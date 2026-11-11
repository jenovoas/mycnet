// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! # mesh_udp — Transporte y Broadcast UDP de Semillas Fractales
//!
//! Encapsula y distribuye datagramas `FractalMeshPacket` (40B) a través de la
//! red mesh (WireGuard / VXLAN / batman-adv) hacia todos los peers configurados.

use crate::state::AppState;
use mycnet_core::mesh::FractalMeshPacket;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

const MAX_PENDING_SEEDS_PER_PEER: usize = 256;

#[derive(Clone)]
struct PendingPacket {
    bytes: [u8; 40],
}

fn poll_capacity(
    peers: &[SocketAddr],
    pending: &HashMap<SocketAddr, VecDeque<PendingPacket>>,
) -> usize {
    peers
        .iter()
        .map(|peer| {
            MAX_PENDING_SEEDS_PER_PEER.saturating_sub(pending.get(peer).map_or(0, VecDeque::len))
        })
        .min()
        .unwrap_or(0)
}

fn window_allows_send(
    window: &mut mycnet_core::predict::SendWindow,
    tick: u64,
    observation: Option<(mycnet_core::s60::SPA, u64)>,
    last_observation_tick: &mut Option<u64>,
) -> bool {
    if let Some((coherence, observed_at)) = observation {
        if (*last_observation_tick).is_none_or(|last| observed_at > last) {
            window.record(observed_at as i64, coherence);
            *last_observation_tick = Some(observed_at);
        }
    }
    window.is_empty() || window.should_send(tick as i64, 10)
}

// Hercules/Ocean son DMZ de cara al exterior, no peers privados de solo envío.
// La restricción de ingreso público se aplica en firewall/VPN, no suprimiendo
// el intercambio entre peers MyCNet de la red privada.

pub async fn start_mesh_udp(
    state: Arc<AppState>,
    bind_addr: &str,
    peers: Vec<SocketAddr>,
) -> anyhow::Result<()> {
    let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
    info!(
        bind = bind_addr,
        peer_count = peers.len(),
        "Mesh UDP socket iniciado para transporte de semillas fractales"
    );

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
                                if let Err(e) =
                                    recv_state.ingest_remote_seed(packet.seed, src).await
                                {
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
        let mut logical_tick = 0u64;
        let mut windows: HashMap<SocketAddr, mycnet_core::predict::SendWindow> = HashMap::new();
        let mut last_observation_tick: HashMap<SocketAddr, Option<u64>> = HashMap::new();
        let mut pending: HashMap<SocketAddr, VecDeque<PendingPacket>> =
            peers.iter().map(|peer| (*peer, VecDeque::new())).collect();

        loop {
            ticker.tick().await;
            logical_tick = logical_tick.wrapping_add(1);

            // Poll consumes ring entries, so never poll more than every peer
            // can retain. A full queue stalls polling/cursor advancement.
            let room = poll_capacity(&peers, &pending).min(32);
            if room > 0 {
                let seeds = send_state.poll_local_fractal_seeds_bounded(room).await;
                for seed in seeds {
                    sequence_id = sequence_id.wrapping_add(1);
                    let packet = FractalMeshPacket::new(sequence_id, seed);
                    let queued = PendingPacket {
                        bytes: packet.to_bytes(),
                    };
                    for queue in pending.values_mut() {
                        queue.push_back(queued.clone());
                    }
                }
            }

            for peer in &peers {
                let queue = pending.get_mut(peer).expect("peer queue initialized");
                while let Some(packet) = queue.front() {
                    let observation = send_state.peer_tq_observation(*peer, logical_tick).await;
                    let win = windows.entry(*peer).or_default();
                    let last_observed = last_observation_tick.entry(*peer).or_default();
                    // Cached TQ carries its original sample tick; only fresh observations enter the OU history.
                    if !window_allows_send(win, logical_tick, observation, last_observed) {
                        break;
                    }
                    match send_socket.send_to(&packet.bytes, peer).await {
                        Ok(_) => {
                            send_state.record_fractal_packet_sent(*peer);
                            queue.pop_front();
                        }
                        Err(e) => {
                            debug!(peer = %peer, error = %e, "Fallo al enviar paquete fractal a peer; pendiente conservado");
                            break;
                        }
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
    use std::sync::atomic::Ordering;
    use tokio::sync::broadcast;

    fn pending_packet(sequence_id: u16) -> PendingPacket {
        let seed = HolographicSeedBinary::from_lfm(u32::from(sequence_id), 0, 0, 0);
        PendingPacket {
            bytes: FractalMeshPacket::new(sequence_id, seed).to_bytes(),
        }
    }

    fn packet_sequence_id(packet: &PendingPacket) -> u16 {
        FractalMeshPacket::from_bytes(&packet.bytes)
            .unwrap()
            .sequence_id
    }

    #[test]
    fn full_peer_queue_stalls_polling_and_retains_head_until_success() {
        let peer: SocketAddr = "127.0.0.1:42429".parse().unwrap();
        let mut pending = HashMap::from([(peer, VecDeque::new())]);
        pending.get_mut(&peer).unwrap().push_back(pending_packet(7));

        assert_eq!(
            poll_capacity(&[peer], &pending),
            MAX_PENDING_SEEDS_PER_PEER - 1
        );
        while pending.get(&peer).unwrap().len() < MAX_PENDING_SEEDS_PER_PEER {
            pending.get_mut(&peer).unwrap().push_back(pending_packet(8));
        }
        assert_eq!(poll_capacity(&[peer], &pending), 0);
        assert_eq!(packet_sequence_id(pending[&peer].front().unwrap()), 7);
        pending.get_mut(&peer).unwrap().pop_front();
        assert_eq!(packet_sequence_id(pending[&peer].front().unwrap()), 8);
    }

    #[test]
    fn rejected_window_keeps_seed_pending() {
        let peer: SocketAddr = "127.0.0.1:42429".parse().unwrap();
        let mut pending = HashMap::from([(peer, VecDeque::new())]);
        pending.get_mut(&peer).unwrap().push_back(pending_packet(9));
        let mut window = mycnet_core::predict::SendWindow::new();
        window.record(9, mycnet_core::s60::SPA::zero());

        assert!(!window.should_send(9, 10));
        assert_eq!(packet_sequence_id(pending[&peer].front().unwrap()), 9);
    }

    #[test]
    fn retained_head_observes_tq_refresh_after_seventeen_loop_ticks_once() {
        let peer: SocketAddr = "127.0.0.1:42429".parse().unwrap();
        let mut pending = HashMap::from([(peer, VecDeque::from([pending_packet(9)]))]);
        let mut window = mycnet_core::predict::SendWindow::new();
        let mut cached_tq = mycnet_core::s60::SPA::zero();
        let mut cache_at = 1u64;
        let mut last_observation_tick = None;
        let mut sends = 0;

        for tick in 1u64..=40 {
            if tick.saturating_sub(cache_at) >= 17 {
                cache_at = tick;
                cached_tq = mycnet_core::s60::SPA::one();
            }
            let queue = pending.get_mut(&peer).unwrap();
            if queue.front().is_some()
                && window_allows_send(
                    &mut window,
                    tick,
                    Some((cached_tq, cache_at)),
                    &mut last_observation_tick,
                )
            {
                assert!(tick >= 18);
                queue.pop_front();
                sends += 1;
            }
            if tick < 18 {
                assert_eq!(window.len(), 1, "caché no debe añadir muestras OU");
            }
        }

        assert_eq!(sends, 1);
        assert!(pending[&peer].is_empty());
    }

    #[test]
    fn test_packet_send_metrics_are_filtered_by_peer() {
        let (tx, _rx) = broadcast::channel(16);
        let metrics_peer: SocketAddr = "127.0.0.1:42429".parse().unwrap();
        let other_peer: SocketAddr = "127.0.0.1:42430".parse().unwrap();
        let state = AppState::new(tx, Some(metrics_peer));

        state.record_fractal_packet_sent(metrics_peer);
        state.record_fractal_packet_sent(other_peer);

        assert_eq!(state.fractal_packets_sent.load(Ordering::Relaxed), 2);
        assert_eq!(
            state.fractal_packets_sent_to_peer.load(Ordering::Relaxed),
            1
        );
    }

    #[tokio::test]
    async fn test_udp_mesh_seed_transmission_roundtrip() {
        let (tx, _rx) = broadcast::channel(16);
        let receiver_addr: SocketAddr = "127.0.0.1:42429".parse().unwrap();
        let sender_socket = UdpSocket::bind("127.0.0.1:42428").await.unwrap();
        let sender_addr = sender_socket.local_addr().unwrap();
        let state = Arc::new(AppState::new(tx, Some(sender_addr)));

        let recv_socket = UdpSocket::bind(receiver_addr).await.unwrap();

        // Enviar paquete fractal
        let seed = HolographicSeedBinary::from_lfm(777, 0, 0, 10);
        let packet = FractalMeshPacket::new(1, seed);
        sender_socket
            .send_to(&packet.to_bytes(), receiver_addr)
            .await
            .unwrap();

        // Recibir y procesar
        let mut buf = [0u8; 128];
        let (len, src) = recv_socket.recv_from(&mut buf).await.unwrap();
        assert_eq!(len, 40);

        let received_packet =
            FractalMeshPacket::from_bytes(&buf[..40].try_into().unwrap()).unwrap();
        assert_eq!(received_packet.seed.token_id, 777);

        let coord = state
            .ingest_remote_seed(received_packet.seed, src)
            .await
            .unwrap();
        assert_eq!(coord.q, 0);
        assert_eq!(coord.r, 0);
        assert_eq!(state.fractal_seeds_ingested.load(Ordering::Relaxed), 1);
        assert_eq!(
            state
                .fractal_seeds_ingested_from_peer
                .load(Ordering::Relaxed),
            1
        );

        let other_src: SocketAddr = "127.0.0.1:42430".parse().unwrap();
        let other_seed = HolographicSeedBinary::from_lfm(778, 0, 0, 10);
        state
            .ingest_remote_seed(other_seed, other_src)
            .await
            .unwrap();
        // Memoria colectiva: la semilla remota aparece en ResonantStore bajo
        // su token_id (proyección holográfica con pérdida, no bytes idénticos).
        let stored = state
            .kv_store
            .lock()
            .await
            .get(&received_packet.seed.token_id.to_le_bytes());
        assert!(
            stored.is_some(),
            "ingest_remote_seed debe expandir la semilla al store local"
        );
        assert_eq!(state.fractal_seeds_ingested.load(Ordering::Relaxed), 2);
        assert_eq!(
            state
                .fractal_seeds_ingested_from_peer
                .load(Ordering::Relaxed),
            1
        );
    }
}
