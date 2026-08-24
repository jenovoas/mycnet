// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! # mycnetd — Daemon principal MycNet
//!
//! - Lee estado batman-adv via `batctl` en tiempo real
//! - Alimenta el ADM con TQ de nodos físicos
//! - Expone API HTTP + WebSocket para consumidores externos

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{info, warn};
use anyhow::Result;

mod api;
mod state;
mod mesh_udp;

use state::AppState;

/// Frecuencia del loop isocrono: ~41.77 Hz (resonancia AXION)
const TICK_HZ: u64 = 42;
const TICK_MS: u64 = 1000 / TICK_HZ;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "mycnetd=info".into())
        )
        .init();

    info!("🍄 MycNet Daemon arrancando...");

    let (tx, _rx) = broadcast::channel::<String>(256);
    let state = Arc::new(AppState::new(tx.clone()));

    // Loop isocrono: batctl → ADM → broadcast
    let state_loop = Arc::clone(&state);
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(TICK_MS));
        loop {
            ticker.tick().await;
            if let Err(e) = state_loop.tick().await {
                warn!("Tick error: {}", e);
            }
        }
    });

    let udp_bind = std::env::var("MYCNET_UDP_BIND")
        .unwrap_or_else(|_| "0.0.0.0:4242".into());

    let peers: Vec<SocketAddr> = std::env::var("MYCNET_PEERS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse::<SocketAddr>().ok())
        .collect();

    let state_udp = Arc::clone(&state);
    let udp_bind_clone = udp_bind.clone();
    tokio::spawn(async move {
        if let Err(e) = mesh_udp::start_mesh_udp(state_udp, &udp_bind_clone, peers).await {
            warn!("Error en transporte Mesh UDP: {}", e);
        }
    });

    // Servidor HTTP + WebSocket
    let bind = std::env::var("MYCNET_BIND")
        .unwrap_or_else(|_| "0.0.0.0:7474".into());

    info!("API escuchando en http://{}", bind);
    let router = api::router(state);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
