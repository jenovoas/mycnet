//! # MycNetClient — Cliente HTTP + WebSocket para el daemon MycNet.

use std::sync::Arc;
use anyhow::{Result, Context};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;
use tracing::{info, warn};

use crate::types::{MeshEvent, MeshSnapshot, NodeInfo};
use crate::MycNetHandler;

/// Cliente para conectarse al daemon MycNet.
///
/// Los programadores instancian este cliente y pasan su implementación
/// de `MycNetHandler` para recibir eventos del mesh.
pub struct MycNetClient {
    base_url: String,
}

impl MycNetClient {
    /// Crea un cliente apuntando al daemon.
    /// Por defecto el daemon escucha en `http://localhost:7474`.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self { base_url: base_url.into() }
    }

    /// Consulta el estado actual del mesh (HTTP GET).
    pub async fn get_snapshot(&self) -> Result<MeshSnapshot> {
        let url = format!("{}/api/v1/adm", self.base_url);
        let body = reqwest::get(url.as_str())
            .await
            .context("No se pudo conectar al daemon MycNet")?
            .text()
            .await?;

        // Parsear respuesta a MeshSnapshot
        let v: serde_json::Value = serde_json::from_str(&body)?;
        Ok(MeshSnapshot {
            tick:      v["tick"].as_u64().unwrap_or(0),
            phase:     v["phase"].as_str().unwrap_or("unknown").to_string(),
            coherence: v["coherence"].as_str().unwrap_or("S60[000;00,00,00,00]").to_string(),
            nodes:     Vec::new(), // Simplificado; parsear nodes si necesario
            ts_unix:   0,
        })
    }

    /// Conecta al stream WebSocket y despacha eventos al handler.
    ///
    /// Esta función no retorna hasta que la conexión se cierra.
    pub async fn connect<H: MycNetHandler>(self, handler: H) -> Result<()> {
        let ws_url = self.base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        let ws_url = format!("{}/api/v1/stream", ws_url);

        info!("Conectando a MycNet daemon: {}", ws_url);
        let (mut ws, _) = connect_async(&ws_url)
            .await
            .context("No se pudo abrir WebSocket con el daemon MycNet")?;

        info!("Conectado al mesh MycNet");

        let handler = Arc::new(handler);
        let mut known_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

        while let Some(msg) = ws.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let v: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v)  => v,
                        Err(e) => { warn!("JSON inválido: {}", e); continue; }
                    };

                    let event = parse_event(&v);

                    // Detectar nodos nuevos/caídos
                    if let Some(nodes) = v["nodes"].as_array() {
                        let current_ids: std::collections::HashSet<String> = nodes.iter()
                            .filter_map(|n| n["physical_id"].as_str().map(String::from))
                            .collect();

                        // Nuevos
                        for id in current_ids.difference(&known_nodes) {
                            handler.on_node_joined(NodeInfo {
                                id:              id.clone(),
                                bat_ip:          String::new(),
                                coherence_s60:   v["coherence"].as_str().unwrap_or("").to_string(),
                                neighbor_count:  0,
                            });
                        }
                        // Caídos
                        for id in known_nodes.difference(&current_ids) {
                            handler.on_node_left(id);
                        }

                        known_nodes = current_ids;
                    }

                    handler.on_mesh_event(event);
                }
                Ok(Message::Close(_)) => {
                    info!("Daemon cerró la conexión");
                    break;
                }
                Err(e) => {
                    warn!("Error WebSocket: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn parse_event(v: &serde_json::Value) -> MeshEvent {
    match v["type"].as_str() {
        Some("mesh_tick") => MeshEvent::Tick {
            tick:      v["tick"].as_u64().unwrap_or(0),
            phase:     v["phase"].as_str().unwrap_or("").to_string(),
            coherence: v["coherence"].as_str().unwrap_or("").to_string(),
            neighbors: v["neighbors"].as_u64().unwrap_or(0) as usize,
        },
        Some("phase_change") => MeshEvent::PhaseChange {
            old_phase: v["old_phase"].as_str().unwrap_or("").to_string(),
            new_phase: v["new_phase"].as_str().unwrap_or("").to_string(),
            factor:    v["factor"].as_u64().unwrap_or(0) as u8,
        },
        Some("low_coherence") => MeshEvent::LowCoherence {
            coherence: v["coherence"].as_str().unwrap_or("").to_string(),
            threshold: v["threshold"].as_str().unwrap_or("").to_string(),
        },
        _ => MeshEvent::Unknown(v.clone()),
    }
}
