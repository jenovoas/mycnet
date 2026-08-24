// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! # mesh_udp — Transporte y Broadcast UDP de Semillas Fractales
//!
//! Encapsula y distribuye datagramas `FractalMeshPacket` (40B) a través de la
//! red mesh (WireGuard / VXLAN / batman-adv) hacia todos los peers configurados.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use mycnet_core::mesh::FractalMeshPacket;
use crate::state::AppState;

pub async fn start_mesh_udp(
    state: Arc<AppState>,
    bind_addr: &str,
    peers: Vec<SocketAddr>,
) -> anyhow::Result<()> {
    let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
    info!(bind = bind_addr, peer_count = peers.len(), "Mesh UDP socket iniciado para transporte de semillas fractales");

    // 1. Tarea de Recepción UDP
    let recv_socket = Arc::clone(&socket);
    let recv_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut buf = [0u8; 128];
        loop {
            match recv_socket.recv_from(&mut buf).await {
                Ok((len, src)) => {
                    if len == 40 {
                        let packet_bytes: [u8; 40] = match buf[..40].try_into() {
                            Ok(b) => b,
                            Err(_) => continue,
                        };
                        match FractalMeshPacket::from_bytes(&packet_bytes) {
                            Ok(packet) => {
                                debug!(src = %src, seq = packet.sequence_id, token_id = packet.seed.token_id, "Semilla fractal recibida vía UDP");
                                if let Err(e) = recv_state.ingest_remote_seed(packet.seed).await {
                                    warn!(error = %e, "Fallo al inyectar semilla remota en LiquidLattice");
                                }
                            }
                            Err(e) => {
                                debug!(src = %src, error = %e, "Paquete fractal descartado (formato o CRC inválido)");
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error en lectura de socket UDP mesh");
                }
            }
        }
    });

    // 2. Tarea de Difusión (Broadcast/Unicast a peers)
    let send_socket = Arc::clone(&socket);
    let send_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_nanos(23_939_835)); // 41.77 Hz
        let mut sequence_id: u16 = 0;

        loop {
            ticker.tick().await;
            let seeds = send_state.poll_local_fractal_seeds().await;
            for seed in seeds {
                sequence_id = sequence_id.wrapping_add(1);
                let packet = FractalMeshPacket::new(sequence_id, seed);
                let bytes = packet.to_bytes();

                for peer in &peers {
                    if let Err(e) = send_socket.send_to(&bytes, peer).await {
                        debug!(peer = %peer, error = %e, "Fallo al enviar paquete fractal a peer");
                    }
                }
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycnet_core::s60::HolographicSeedBinary;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_udp_mesh_seed_transmission_roundtrip() {
        let (tx, _rx) = broadcast::channel(16);
        let state = Arc::new(AppState::new(tx));

        let receiver_addr: SocketAddr = "127.0.0.1:42429".parse().unwrap();
        let sender_socket = UdpSocket::bind("127.0.0.1:42428").await.unwrap();

        let recv_socket = UdpSocket::bind(receiver_addr).await.unwrap();

        // Enviar paquete fractal
        let seed = HolographicSeedBinary::from_lfm(777, 0, 0, 10);
        let packet = FractalMeshPacket::new(1, seed);
        sender_socket.send_to(&packet.to_bytes(), receiver_addr).await.unwrap();

        // Recibir y procesar
        let mut buf = [0u8; 128];
        let (len, _) = recv_socket.recv_from(&mut buf).await.unwrap();
        assert_eq!(len, 40);

        let received_packet = FractalMeshPacket::from_bytes(&buf[..40].try_into().unwrap()).unwrap();
        assert_eq!(received_packet.seed.token_id, 777);

        let coord = state.ingest_remote_seed(received_packet.seed).await.unwrap();
        assert_eq!(coord.q, 0);
        assert_eq!(coord.r, 0);
    }
}
