// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! # API HTTP + WebSocket del daemon MycNet
//!
//! Endpoints:
//!   GET  /health          — estado básico
//!   GET  /api/v1/mesh     — snapshot completo del mesh
//!   GET  /api/v1/adm      — estado interno del ADM
//!   WS   /api/v1/stream   — telemetría en tiempo real

use axum::{
    Router,
    routing::get,
    extract::{State, WebSocketUpgrade},
    extract::ws::{WebSocket, Message},
    response::{Json, IntoResponse},
};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

use crate::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health",          get(health))
        .route("/api/v1/mesh",     get(mesh_state))
        .route("/api/v1/adm",      get(adm_state))
        .route("/api/v1/stream",   get(ws_handler))
        .with_state(state)
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let (tick, nodes) = {
        let adm = state.adm.lock().await;
        (adm.tick_count, adm.nodes.len())
    };
    let fractal_source_connected = state.fractal_bridge.lock().await.is_some();

    Json(json!({
        "status": "OK",
        "tick": tick,
        "nodes": nodes,
        "phase": mycnet_core::yhwh::YhwhPhase::current().name(),
        "fractal_source_connected": fractal_source_connected,
        "fractal_packet_bytes": std::mem::size_of::<mycnet_core::mesh::FractalMeshPacket>(),
        "fractal_metrics_peer": state.metrics_peer.map(|peer| peer.to_string()),
        "fractal_packets_sent": state.fractal_packets_sent.load(Ordering::Relaxed),
        "fractal_packets_sent_to_peer": state.fractal_packets_sent_to_peer.load(Ordering::Relaxed),
        "fractal_seeds_ingested": state.fractal_seeds_ingested.load(Ordering::Relaxed),
        "fractal_seeds_ingested_from_peer": state.fractal_seeds_ingested_from_peer.load(Ordering::Relaxed),
    }))
}

async fn mesh_state(State(state): State<Arc<AppState>>) -> Json<Value> {
    let mesh = state.get_mesh_state().await;
    Json(serde_json::to_value(mesh).unwrap_or_default())
}

async fn adm_state(State(state): State<Arc<AppState>>) -> Json<Value> {
    let adm = state.adm.lock().await;
    Json(json!({
        "tick":       adm.tick_count,
        "coherence":  adm.coherence().to_string(),
        "nodes":      adm.snapshot(),
    }))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();
    info!("Cliente WebSocket conectado");

    loop {
        match rx.recv().await {
            Ok(msg) => {
                if socket.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("WS client lagged {} messages", n);
            }
            Err(_) => break,
        }
    }

    info!("Cliente WebSocket desconectado");
}
