//! # mycnetd — Daemon principal MycNet
//!
//! - Lee estado batman-adv via `batctl` en tiempo real
//! - Alimenta el ADM con TQ de nodos físicos
//! - Expone API HTTP + WebSocket para consumidores externos

use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tokio::time::{interval, Duration};
use tracing::{info, warn};
use anyhow::Result;

mod api;
mod state;

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

    // Servidor HTTP + WebSocket
    let bind = std::env::var("MYCNET_BIND")
        .unwrap_or_else(|_| "0.0.0.0:7474".into());

    info!("API escuchando en http://{}", bind);
    let router = api::router(state);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
