// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! # API HTTP + WebSocket del daemon MycNet
//!
//! Endpoints:
//!   GET  /health          — estado básico
//!   GET  /api/v1/kv/:token  — presencia de semilla en ResonantStore (debug)
//!   GET  /api/v1/kv-metadata?key=… — metadatos KV exactos locales, sin payload

use axum::{
    extract::ws::{Message, WebSocket},
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

use crate::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/mesh", get(mesh_state))
        .route("/api/v1/adm", get(adm_state))
        .route("/api/v1/kv/:token", get(kv_lookup))
        .route("/api/v1/kv-metadata", get(kv_metadata))
        .route("/api/v1/stream", get(ws_handler))
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
        "fractal_local_polled": state.fractal_local_polled.load(Ordering::Relaxed),
    }))
}

async fn mesh_state(State(state): State<Arc<AppState>>) -> Json<Value> {
    let mesh = state.get_mesh_state().await;
    Json(serde_json::to_value(mesh).unwrap_or_default())
}

async fn kv_lookup(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(token): axum::extract::Path<u32>,
) -> impl IntoResponse {
    let key = token.to_le_bytes();
    let store = state.kv_store.lock().await;
    let present = store.get(&key).is_some();
    Json(json!({ "token_id": token, "present": present }))
}

#[derive(Deserialize)]
struct KvQuery {
    key: String,
}

async fn kv_metadata(
    State(state): State<Arc<AppState>>,
    Query(query): Query<KvQuery>,
) -> impl IntoResponse {
    if !mycnet_memory::storage::kv_key_allowed(&query.key) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "clave fuera de whitelist" })),
        );
    }
    let store = state.kv_store.lock().await;
    let metadata = match store.get_kv_seed(&query.key) {
        Some(record) => json!({
            "key": query.key,
            "present": true,
            "timestamp_ns": record.timestamp_ns,
            "seed_bytes": record.seed_bytes,
            "payload_hash": record.payload_hash,
            "payload_available": false,
        }),
        None => json!({
            "key": query.key,
            "present": false,
            "payload_available": false,
        }),
    };
    (StatusCode::OK, Json(metadata))
}

async fn adm_state(State(state): State<Arc<AppState>>) -> Json<Value> {
    let adm = state.adm.lock().await;
    Json(json!({
        "tick":       adm.tick_count,
        "coherence":  adm.coherence().to_string(),
        "nodes":      adm.snapshot(),
    }))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

#[cfg(test)]
mod tests {
    use super::*;
    use mycnet_core::s60::HolographicSeedBinary;
    use mycnet_memory::storage::KvSeedRecord;

    #[tokio::test]
    async fn metadata_lookup_exposes_only_exact_local_metadata() {
        let (tx, _rx) = broadcast::channel(16);
        let state = Arc::new(AppState::new(tx, None));
        let record = KvSeedRecord {
            timestamp_ns: 123456789,
            payload_hash: [23; 32],
            seed_bytes: HolographicSeedBinary::from_lfm(12, 0, 0, 5).to_bytes(),
        };
        state.ingest_kv_seed("crystal-api", record).await.unwrap();
        let response = kv_metadata(
            State(Arc::clone(&state)),
            Query(KvQuery {
                key: "crystal-api".to_owned(),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["present"], true);
        assert_eq!(value["timestamp_ns"], record.timestamp_ns);
        assert_eq!(value["seed_bytes"], json!(record.seed_bytes));
        assert_eq!(value["payload_hash"], json!(record.payload_hash));
        assert_eq!(value["payload_available"], false);
        assert!(value.get("payload").is_none());

        let missing = kv_metadata(
            State(Arc::clone(&state)),
            Query(KvQuery {
                key: "crystal-missing".to_owned(),
            }),
        )
        .await
        .into_response();
        let bytes = axum::body::to_bytes(missing.into_body(), 4096)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["present"], false);
        assert_eq!(value["payload_available"], false);

        let forbidden = kv_metadata(
            State(state),
            Query(KvQuery {
                key: "security-secret".to_owned(),
            }),
        )
        .await
        .into_response();
        assert_eq!(forbidden.status(), StatusCode::BAD_REQUEST);
    }
}
