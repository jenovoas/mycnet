//! Estado compartido del daemon.

use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use serde_json::json;
use anyhow::Result;
use tracing::info;

use mycnet_core::{ADM, MeshState, PhysicalNode};
use mycnet_core::mesh::{get_local_neighbors, mesh_coherence};
use mycnet_core::yhwh::YhwhPhase;

pub struct AppState {
    pub adm:      Mutex<ADM>,
    pub tx:       broadcast::Sender<String>,
    pub hostname: String,
    pub bat_ip:   String,
}

impl AppState {
    pub fn new(tx: broadcast::Sender<String>) -> Self {
        let mut adm = ADM::new();

        // Red hexagonal de 6 nodos (topología estándar MycNet)
        // Anillo hexagonal alrededor del origen
        let hex_ring = [
            (1, 0), (1, -1), (0, -1),
            (-1, 0), (-1, 1), (0, 1),
        ];
        adm.add_node(0, 0); // nodo central
        for (q, r) in hex_ring {
            adm.add_node(q, r);
        }

        let hostname = hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let bat_ip = std::env::var("MYCNET_BAT_IP")
            .unwrap_or_else(|_| "10.10.0.11".into());

        Self { adm: Mutex::new(adm), tx, hostname, bat_ip }
    }

    /// Tick principal: lee batctl → actualiza ADM → broadcast.
    pub async fn tick(&self) -> Result<()> {
        let phase = YhwhPhase::current();

        // Leer vecinos reales de batman-adv
        let neighbors = match get_local_neighbors() {
            Ok(n) => n,
            Err(e) => {
                // Sin batctl disponible (dev mode): simular
                tracing::debug!("batctl no disponible ({}), modo simulación", e);
                Vec::new()
            }
        };

        let coherence = mesh_coherence(&neighbors);

        // Actualizar ADM con TQ de vecinos reales
        {
            let mut adm = self.adm.lock().await;

            // Inyectar TQ de cada vecino en su nodo ADM correspondiente
            for (i, neighbor) in neighbors.iter().enumerate() {
                let ring_positions = [
                    (1i64, 0i64), (1, -1), (0, -1),
                    (-1, 0), (-1, 1), (0, 1),
                ];
                if let Some(&(q, r)) = ring_positions.get(i) {
                    adm.inject_tq(q, r, neighbor.tq);
                }
            }

            adm.tick();

            // Broadcast snapshot
            let snapshot = json!({
                "type":       "mesh_tick",
                "tick":       adm.tick_count,
                "phase":      phase.name(),
                "coherence":  coherence.to_string(),
                "neighbors":  neighbors.len(),
                "nodes":      adm.snapshot(),
            });

            let _ = self.tx.send(snapshot.to_string());
        }

        Ok(())
    }

    pub async fn get_mesh_state(&self) -> MeshState {
        let neighbors = get_local_neighbors().unwrap_or_default();
        let coherence = mesh_coherence(&neighbors);

        MeshState {
            nodes: vec![PhysicalNode {
                id:              self.hostname.clone(),
                bat_ip:          self.bat_ip.clone(),
                mesh_coherence:  coherence,
                neighbors,
            }],
            ts_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

// Pequeña dependencia para hostname
mod hostname {
    pub fn get() -> Result<std::ffi::OsString, ()> {
        std::process::Command::new("hostname")
            .output()
            .map(|o| std::ffi::OsString::from(
                String::from_utf8_lossy(&o.stdout).trim().to_string()
            ))
            .map_err(|_| ())
    }
}
