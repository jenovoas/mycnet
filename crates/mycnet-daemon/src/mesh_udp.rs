// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! # mesh_udp — Transporte y Broadcast UDP de Semillas Fractales
//!
//! Encapsula y distribuye datagramas `FractalMeshPacket` (40B) a través de la
//! red mesh (WireGuard / VXLAN / batman-adv) hacia todos los peers configurados.

use crate::state::AppState;
use mycnet_core::mesh::FractalMeshPacket;
use mycnet_memory::storage::{kv_key_allowed, KvSeedRecord, MAX_KV_KEY_BYTES};
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

const MAX_PENDING_SEEDS_PER_PEER: usize = 256;
const KV_HEADER_BYTES: usize = 80;
const MAX_KV_PACKET_BYTES: usize = KV_HEADER_BYTES + MAX_KV_KEY_BYTES;
const MAX_PENDING_KV: usize = 256;
const KV_RETRY_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum KvKind {
    Publish = 1,
    Replicate = 2,
    Ack = 3,
    Committed = 4,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct KvIdentity {
    key: String,
    record: KvSeedRecord,
}

#[derive(Debug, PartialEq, Eq)]
struct KvFrame {
    kind: KvKind,
    identity: KvIdentity,
}

struct KvDatagram {
    bytes: [u8; MAX_KV_PACKET_BYTES],
    len: usize,
}

impl KvDatagram {
    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl KvFrame {
    fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if !(KV_HEADER_BYTES + 1..=MAX_KV_PACKET_BYTES).contains(&bytes.len()) {
            return Err("Tamaño MYKV inválido");
        }
        if &bytes[..4] != b"MYKV" || bytes[5] != 1 {
            return Err("Magic o versión MYKV inválidos");
        }
        let kind = match bytes[4] {
            1 => KvKind::Publish,
            2 => KvKind::Replicate,
            3 => KvKind::Ack,
            4 => KvKind::Committed,
            _ => return Err("Tipo MYKV inválido"),
        };
        let key_len = u16::from_le_bytes(bytes[6..8].try_into().unwrap()) as usize;
        if bytes.len() != KV_HEADER_BYTES + key_len {
            return Err("Longitud de clave MYKV inválida");
        }
        let key =
            std::str::from_utf8(&bytes[KV_HEADER_BYTES..]).map_err(|_| "Clave MYKV no UTF-8")?;
        if !kv_key_allowed(key) {
            return Err("Clave MYKV fuera de whitelist");
        }
        let record = KvSeedRecord {
            timestamp_ns: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            seed_bytes: bytes[16..48].try_into().unwrap(),
            payload_hash: bytes[48..80].try_into().unwrap(),
        };
        record.seed()?;
        Ok(Self {
            kind,
            identity: KvIdentity {
                key: key.to_owned(),
                record,
            },
        })
    }

    fn encode(&self, kind: KvKind) -> KvDatagram {
        let key = self.identity.key.as_bytes();
        let mut bytes = [0u8; MAX_KV_PACKET_BYTES];
        bytes[..4].copy_from_slice(b"MYKV");
        bytes[4] = kind as u8;
        bytes[5] = 1;
        bytes[6..8].copy_from_slice(&(key.len() as u16).to_le_bytes());
        bytes[8..16].copy_from_slice(&self.identity.record.timestamp_ns.to_le_bytes());
        bytes[16..48].copy_from_slice(&self.identity.record.seed_bytes);
        bytes[48..80].copy_from_slice(&self.identity.record.payload_hash);
        bytes[KV_HEADER_BYTES..KV_HEADER_BYTES + key.len()].copy_from_slice(key);
        KvDatagram {
            bytes,
            len: KV_HEADER_BYTES + key.len(),
        }
    }
}

struct PendingKv {
    origin: SocketAddr,
    awaiting: HashSet<SocketAddr>,
    replicate: KvDatagram,
}

/// Estado volátil acotado: cada publicación ocupa como máximo un slot por
/// peer y un solo destinatario local. No hay caché de completions ilimitada.
struct KvDelivery {
    peers: HashSet<SocketAddr>,
    pending: HashMap<KvIdentity, PendingKv>,
}

impl KvDelivery {
    fn new(peers: &[SocketAddr]) -> Self {
        Self {
            peers: peers.iter().copied().collect(),
            pending: HashMap::new(),
        }
    }

    fn authorized(&self, kind: KvKind, source: SocketAddr) -> bool {
        match kind {
            KvKind::Publish => source.ip().is_loopback(),
            KvKind::Replicate | KvKind::Ack => self.peers.contains(&source),
            KvKind::Committed => false,
        }
    }

    async fn send_pending(socket: &UdpSocket, pending: &PendingKv) {
        for peer in &pending.awaiting {
            if let Err(error) = socket.send_to(pending.replicate.as_bytes(), peer).await {
                debug!(%peer, %error, "MYKV pendiente conservado tras fallo de envío");
            }
        }
    }

    async fn retry(&self, socket: &UdpSocket) {
        for pending in self.pending.values() {
            Self::send_pending(socket, pending).await;
        }
    }

    async fn handle(
        &mut self,
        state: &AppState,
        socket: &UdpSocket,
        frame: KvFrame,
        source: SocketAddr,
    ) {
        if !self.authorized(frame.kind, source) {
            return;
        }
        match frame.kind {
            KvKind::Publish => {
                if self.peers.is_empty() {
                    return; // Sin peer no hay entrega colectiva ni Committed.
                }
                if let Some(pending) = self.pending.get_mut(&frame.identity) {
                    // Un retry puede usar un nuevo socket; no acumular waiters.
                    pending.origin = source;
                    return;
                }
                if self.pending.len() >= MAX_PENDING_KV {
                    return; // Backpressure: el cliente conserva y reintenta.
                }
                if let Err(error) = state
                    .ingest_kv_seed(&frame.identity.key, frame.identity.record)
                    .await
                {
                    debug!(%error, "Publicación MYKV no aplicada; sin confirmación");
                    return;
                }
                let pending = PendingKv {
                    origin: source,
                    awaiting: self.peers.clone(),
                    replicate: frame.encode(KvKind::Replicate),
                };
                Self::send_pending(socket, &pending).await;
                self.pending.insert(frame.identity, pending);
            }
            KvKind::Replicate => {
                // Applied, Duplicate y Superseded son aceptaciones LWW;
                // errores de almacenamiento nunca producen ACK.
                if state
                    .ingest_kv_seed(&frame.identity.key, frame.identity.record)
                    .await
                    .is_ok()
                {
                    let ack = frame.encode(KvKind::Ack);
                    if let Err(error) = socket.send_to(ack.as_bytes(), source).await {
                        debug!(%error, "ACK MYKV perdido; el peer reintentará");
                    }
                }
            }
            KvKind::Ack => {
                let complete = self
                    .pending
                    .get_mut(&frame.identity)
                    .is_some_and(|pending| {
                        pending.awaiting.remove(&source);
                        pending.awaiting.is_empty()
                    });
                if complete {
                    let pending = self
                        .pending
                        .remove(&frame.identity)
                        .expect("pendiente completo");
                    let committed = frame.encode(KvKind::Committed);
                    // Si se pierde, el Publish idéntico reinicia ACK de todos
                    // los peers; la versión local no vuelve a inyectarse.
                    if let Err(error) = socket.send_to(committed.as_bytes(), pending.origin).await {
                        debug!(%error, "Committed MYKV perdido; el cliente reintentará");
                    }
                }
            }
            KvKind::Committed => {}
        }
    }
}

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
    let recv_peers = peers.clone();
    tokio::spawn(async move {
        // Un byte extra permite detectar datagramas truncados por exceso.
        let mut buf = [0u8; MAX_KV_PACKET_BYTES + 1];
        let mut kv = KvDelivery::new(&recv_peers);
        let mut retry = interval(KV_RETRY_INTERVAL);
        loop {
            let received = tokio::select! {
                received = recv_socket.recv_from(&mut buf) => received,
                _ = retry.tick() => {
                    kv.retry(&recv_socket).await;
                    continue;
                }
            };
            match received {
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
                    } else if let Ok(frame) = KvFrame::decode(&buf[..len]) {
                        kv.handle(&recv_state, &recv_socket, frame, src).await;
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

#[cfg(test)]
mod kv_tests {
    use super::*;
    use mycnet_core::s60::{HolographicSeedBinary, SPA};
    use mycnet_memory::lattice::Coord;
    use tokio::sync::broadcast;
    use tokio::time::timeout;

    fn frame(kind: KvKind, timestamp_ns: u64) -> KvFrame {
        KvFrame {
            kind,
            identity: KvIdentity {
                key: "crystal-transport".to_owned(),
                record: KvSeedRecord {
                    timestamp_ns,
                    payload_hash: [17; 32],
                    seed_bytes: HolographicSeedBinary::from_lfm(123, 0, 0, 7).to_bytes(),
                },
            },
        }
    }

    async fn fixture(
        peer_count: usize,
    ) -> (AppState, UdpSocket, UdpSocket, Vec<UdpSocket>, KvDelivery) {
        let (tx, _rx) = broadcast::channel(16);
        let state = AppState::new(tx, None);
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let mut peers = Vec::new();
        for _ in 0..peer_count {
            peers.push(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        }
        let addresses: Vec<_> = peers
            .iter()
            .map(|peer| peer.local_addr().unwrap())
            .collect();
        (state, socket, client, peers, KvDelivery::new(&addresses))
    }

    async fn receive(socket: &UdpSocket) -> KvFrame {
        let mut bytes = [0u8; MAX_KV_PACKET_BYTES + 1];
        let (len, _) = timeout(Duration::from_secs(1), socket.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        KvFrame::decode(&bytes[..len]).unwrap()
    }

    async fn assert_silent(socket: &UdpSocket) {
        let mut bytes = [0u8; MAX_KV_PACKET_BYTES + 1];
        assert!(
            timeout(Duration::from_millis(20), socket.recv_from(&mut bytes))
                .await
                .is_err()
        );
    }

    #[test]
    fn mykv_roundtrip_preserves_full_identity_and_wire_offsets() {
        let mut original = frame(KvKind::Publish, 0x0102_0304_0506_0708);
        original.identity.record.seed_bytes[30..].copy_from_slice(&[0xab, 0xcd]);
        for kind in [
            KvKind::Publish,
            KvKind::Replicate,
            KvKind::Ack,
            KvKind::Committed,
        ] {
            let encoded = original.encode(kind);
            let bytes = encoded.as_bytes();
            assert_eq!(&bytes[..4], b"MYKV");
            assert_eq!(bytes[4], kind as u8);
            assert_eq!(bytes[5], 1);
            assert_eq!(
                &bytes[8..16],
                &original.identity.record.timestamp_ns.to_le_bytes()
            );
            assert_eq!(&bytes[16..48], &original.identity.record.seed_bytes);
            assert_eq!(&bytes[48..80], &original.identity.record.payload_hash);
            let decoded = KvFrame::decode(bytes).unwrap();
            assert_eq!(decoded.kind, kind);
            assert_eq!(decoded.identity, original.identity);
        }
        original.identity.key = format!("crystal-{}", "x".repeat(MAX_KV_KEY_BYTES - 8));
        let encoded = original.encode(KvKind::Publish);
        assert_eq!(encoded.len, 592);
        assert_eq!(
            KvFrame::decode(encoded.as_bytes()).unwrap().identity,
            original.identity
        );
    }

    #[test]
    fn mykv_rejects_bad_format_crc_and_non_whitelisted_keys() {
        let valid = frame(KvKind::Publish, 17).encode(KvKind::Publish);
        for offset in [0, 4, 5, 6, 16] {
            let mut bad = valid.as_bytes().to_vec();
            bad[offset] ^= 0xff;
            assert!(KvFrame::decode(&bad).is_err(), "offset {offset}");
        }
        assert!(KvFrame::decode(&valid.as_bytes()[..80]).is_err());
        let mut extra = valid.as_bytes().to_vec();
        extra.push(0);
        assert!(KvFrame::decode(&extra).is_err());
        assert!(KvFrame::decode(&[0; MAX_KV_PACKET_BYTES + 1]).is_err());
        let mut bad_utf8 = valid.as_bytes().to_vec();
        bad_utf8[80] = 0xff;
        assert!(KvFrame::decode(&bad_utf8).is_err());
        for key in ["security-secret", "genesis", "portal", ""] {
            let mut bad = frame(KvKind::Publish, 17);
            bad.identity.key = key.to_owned();
            assert!(KvFrame::decode(bad.encode(KvKind::Publish).as_bytes()).is_err());
        }
        let mut allowed = frame(KvKind::Publish, 17);
        allowed.identity.key = "portal-metrics-node".to_owned();
        assert!(KvFrame::decode(allowed.encode(KvKind::Publish).as_bytes()).is_ok());

        // Nunca desbordar i32 al verificar coordenadas recibidas.
        let mut overflow = valid.as_bytes().to_vec();
        overflow[20..24].copy_from_slice(&i32::MAX.to_le_bytes());
        overflow[24..28].copy_from_slice(&i32::MAX.to_le_bytes());
        assert!(KvFrame::decode(&overflow).is_err());
        for (q, r, s) in [(i32::MAX, i32::MAX, 2), (i32::MAX, 1, i32::MIN)] {
            let mut extreme = HolographicSeedBinary::from_lfm(1, 0, 0, 0);
            extreme.coord_q = q;
            extreme.coord_r = r;
            extreme.coord_s = s;
            extreme.crc16 = extreme.compute_checksum();
            assert!(extreme.is_valid_crc());
            overflow[16..48].copy_from_slice(&extreme.to_bytes());
            assert!(KvFrame::decode(&overflow).is_err());
        }
    }

    #[tokio::test]
    async fn mykv_commit_requires_exact_ack_from_every_configured_peer() {
        let (state, socket, client, peers, mut delivery) = fixture(2).await;
        let origin = client.local_addr().unwrap();
        let published = frame(KvKind::Publish, 17);
        let identity = published.identity.clone();
        delivery.handle(&state, &socket, published, origin).await;
        for peer in &peers {
            assert_eq!(receive(peer).await.identity, identity);
        }
        assert_eq!(
            state.kv_store.lock().await.get_kv_seed(&identity.key),
            Some(&identity.record)
        );
        assert_silent(&client).await;

        let mut wrong_ack = frame(KvKind::Ack, 17);
        wrong_ack.identity.record.payload_hash[0] ^= 1;
        delivery
            .handle(&state, &socket, wrong_ack, peers[0].local_addr().unwrap())
            .await;
        delivery
            .handle(&state, &socket, frame(KvKind::Ack, 17), origin)
            .await;
        assert_eq!(delivery.pending[&identity].awaiting.len(), 2);
        delivery
            .handle(
                &state,
                &socket,
                frame(KvKind::Ack, 17),
                peers[0].local_addr().unwrap(),
            )
            .await;
        delivery
            .handle(
                &state,
                &socket,
                frame(KvKind::Ack, 17),
                peers[0].local_addr().unwrap(),
            )
            .await;
        assert_silent(&client).await;
        assert_eq!(delivery.pending[&identity].awaiting.len(), 1);
        delivery
            .handle(
                &state,
                &socket,
                frame(KvKind::Ack, 17),
                peers[1].local_addr().unwrap(),
            )
            .await;
        let committed = receive(&client).await;
        assert_eq!(committed.kind, KvKind::Committed);
        assert_eq!(committed.identity, identity);
        assert!(delivery.pending.is_empty());
    }

    #[tokio::test]
    async fn mykv_loss_retries_original_identity_without_reinjection() {
        let (state, socket, client, peers, mut delivery) = fixture(1).await;
        let origin = client.local_addr().unwrap();
        delivery
            .handle(&state, &socket, frame(KvKind::Publish, 17), origin)
            .await;
        let lost = receive(&peers[0]).await; // Primera réplica perdida.
        let coord = Coord { q: 0, r: 0 };
        state
            .kv_store
            .lock()
            .await
            .lattice
            .nodes
            .get_mut(&coord)
            .unwrap()
            .amplitude = SPA::from_raw(1);
        delivery
            .handle(&state, &socket, frame(KvKind::Publish, 17), origin)
            .await;
        assert_eq!(delivery.pending.len(), 1);
        delivery.retry(&socket).await;
        assert_eq!(receive(&peers[0]).await, lost);
        assert_eq!(
            state.kv_store.lock().await.lattice.nodes[&coord].amplitude,
            SPA::from_raw(1)
        );
        delivery
            .handle(
                &state,
                &socket,
                frame(KvKind::Ack, 17),
                peers[0].local_addr().unwrap(),
            )
            .await;
        let lost_commit = receive(&client).await; // Committed perdido.
        assert_eq!(lost_commit.kind, KvKind::Committed);

        delivery
            .handle(&state, &socket, frame(KvKind::Publish, 17), origin)
            .await;
        assert_eq!(receive(&peers[0]).await, lost);
        assert_silent(&client).await; // Revalidar peer, no éxito por caché.
        delivery
            .handle(
                &state,
                &socket,
                frame(KvKind::Ack, 17),
                peers[0].local_addr().unwrap(),
            )
            .await;
        assert_eq!(receive(&client).await, lost_commit);
        assert_eq!(
            state.kv_store.lock().await.lattice.nodes[&coord].amplitude,
            SPA::from_raw(1)
        );
    }

    #[tokio::test]
    async fn mykv_replica_ack_follows_store_and_reordering_converges() {
        let (state, socket, _client, peers, mut delivery) = fixture(1).await;
        let source = peers[0].local_addr().unwrap();
        let newer = frame(KvKind::Replicate, 20);
        let expected = newer.identity.clone();
        delivery.handle(&state, &socket, newer, source).await;
        // El ACK observable corresponde a una versión ya almacenada.
        assert_eq!(
            state.kv_store.lock().await.get_kv_seed(&expected.key),
            Some(&expected.record)
        );
        assert_eq!(receive(&peers[0]).await.kind, KvKind::Ack);
        let coord = Coord { q: 0, r: 0 };
        state
            .kv_store
            .lock()
            .await
            .lattice
            .nodes
            .get_mut(&coord)
            .unwrap()
            .amplitude = SPA::from_raw(2);
        for timestamp in [10, 20, 10] {
            delivery
                .handle(&state, &socket, frame(KvKind::Replicate, timestamp), source)
                .await;
            assert_eq!(receive(&peers[0]).await.kind, KvKind::Ack);
            assert_eq!(
                state.kv_store.lock().await.get_kv_seed(&expected.key),
                Some(&expected.record)
            );
            assert_eq!(
                state.kv_store.lock().await.lattice.nodes[&coord].amplitude,
                SPA::from_raw(2)
            );
        }
        let mut impossible = frame(KvKind::Replicate, 30);
        impossible.identity.record.seed_bytes =
            HolographicSeedBinary::from_lfm(1, 9, 0, 0).to_bytes();
        delivery.handle(&state, &socket, impossible, source).await;
        assert_silent(&peers[0]).await;
        assert_eq!(
            state.kv_store.lock().await.get_kv_seed(&expected.key),
            Some(&expected.record)
        );
    }

    #[tokio::test]
    async fn mykv_requires_loopback_publish_peers_for_replication_and_nonempty_peers() {
        let (state, socket, client, peers, mut delivery) = fixture(1).await;
        assert!(!delivery.authorized(KvKind::Publish, "192.0.2.1:4242".parse().unwrap()));
        assert!(!delivery.authorized(KvKind::Replicate, client.local_addr().unwrap()));
        assert!(!delivery.authorized(KvKind::Ack, "127.0.0.1:1".parse().unwrap()));
        assert!(!delivery.authorized(KvKind::Committed, peers[0].local_addr().unwrap()));
        delivery
            .handle(
                &state,
                &socket,
                frame(KvKind::Replicate, 17),
                client.local_addr().unwrap(),
            )
            .await;
        assert!(state
            .kv_store
            .lock()
            .await
            .get_kv_seed("crystal-transport")
            .is_none());
        assert_silent(&client).await;
        let mut no_peers = KvDelivery::new(&[]);
        no_peers
            .handle(
                &state,
                &socket,
                frame(KvKind::Publish, 17),
                client.local_addr().unwrap(),
            )
            .await;
        assert!(no_peers.pending.is_empty());
        assert!(state
            .kv_store
            .lock()
            .await
            .get_kv_seed("crystal-transport")
            .is_none());
        assert_silent(&client).await;
    }

    #[tokio::test]
    async fn mykv_full_pending_backpressures_without_store_or_unbounded_waiters() {
        let (state, socket, client, peers, mut delivery) = fixture(1).await;
        let source = client.local_addr().unwrap();
        for timestamp in 0..MAX_PENDING_KV as u64 {
            let queued = frame(KvKind::Publish, timestamp);
            delivery.pending.insert(
                queued.identity.clone(),
                PendingKv {
                    origin: source,
                    awaiting: delivery.peers.clone(),
                    replicate: queued.encode(KvKind::Replicate),
                },
            );
        }
        delivery
            .handle(&state, &socket, frame(KvKind::Publish, 999), source)
            .await;
        assert_eq!(delivery.pending.len(), MAX_PENDING_KV);
        assert!(state
            .kv_store
            .lock()
            .await
            .get_kv_seed("crystal-transport")
            .is_none());
        assert_silent(&client).await;
        let replacement = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        delivery
            .handle(
                &state,
                &socket,
                frame(KvKind::Publish, 0),
                replacement.local_addr().unwrap(),
            )
            .await;
        assert_eq!(delivery.pending.len(), MAX_PENDING_KV);
        assert_eq!(
            delivery.pending[&frame(KvKind::Publish, 0).identity].origin,
            replacement.local_addr().unwrap()
        );
        assert!(delivery
            .pending
            .values()
            .all(|pending| pending.awaiting.len() == peers.len()));
    }
}

#[cfg(test)]
mod kv_capacity_tests {
    use super::*;
    use mycnet_core::s60::HolographicSeedBinary;
    use mycnet_memory::storage::MAX_KV_KEYS;
    use tokio::sync::broadcast;
    use tokio::time::timeout;

    #[tokio::test]
    async fn full_store_never_acks_unstored_key_but_acks_existing_key() {
        let (tx, _rx) = broadcast::channel(16);
        let state = AppState::new(tx, None);
        let record = KvSeedRecord {
            timestamp_ns: 1,
            payload_hash: [1; 32],
            seed_bytes: HolographicSeedBinary::from_lfm(1, 0, 0, 0).to_bytes(),
        };
        {
            let mut store = state.kv_store.lock().await;
            for index in 0..MAX_KV_KEYS {
                store
                    .put_kv_seed(&format!("crystal-{index}"), record)
                    .unwrap();
            }
        }
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let peer = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let source = peer.local_addr().unwrap();
        let mut delivery = KvDelivery::new(&[source]);
        let mut received = [0u8; MAX_KV_PACKET_BYTES + 1];
        let overflow = KvFrame {
            kind: KvKind::Replicate,
            identity: KvIdentity {
                key: "crystal-overflow".to_owned(),
                record,
            },
        };
        delivery.handle(&state, &socket, overflow, source).await;
        assert!(
            timeout(Duration::from_millis(20), peer.recv_from(&mut received))
                .await
                .is_err()
        );
        assert!(state
            .kv_store
            .lock()
            .await
            .get_kv_seed("crystal-overflow")
            .is_none());
        let existing = KvFrame {
            kind: KvKind::Replicate,
            identity: KvIdentity {
                key: "crystal-0".to_owned(),
                record,
            },
        };
        delivery.handle(&state, &socket, existing, source).await;
        let (len, _) = timeout(Duration::from_secs(1), peer.recv_from(&mut received))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(KvFrame::decode(&received[..len]).unwrap().kind, KvKind::Ack);
    }
}
