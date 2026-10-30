// Autor: Jaime Novoa Sepúlveda — Todos los derechos reservados.
//! guardian-uplink — Adaptador WAL Guardian → mesh MyCNet (Marte → Fenix).
//!
//! Lee el WAL del security-gateway (`guardian_security_wal.jsonl`) en tail,
//! convierte cada evento eBPF a `HolographicSeedBinary` y lo envía por UDP
//! al peer MyCNet (Fenix). Sin HTTP, sin tokens: el dato canta en el lattice.
//!
//! Mapeo evento → seed (mismas reglas que `CrystalRuntime::publish_fractal_seed`):
//! - token_id: hash del event_id (u32).
//! - coord (q,r,s): hexágono del índice = pid % 37 nodos (malla 3 anillos).
//! - amplitude_s60: entropy_s60_raw del evento.
//! - phase_sector: severity % 60. yhwh_pulse: guardian_code. flags: 0x01.
//! - crc16: calculado por `HolographicSeedBinary::new`.
//!
//! Env:
//! - GUARDIAN_WAL (default /var/log/sentinel/guardian_security_wal.jsonl)
//! - MYCNET_UPLINK_PEER (default 10.88.0.1:4242)
//! - MYCNET_UPLINK_BIND (default 0.0.0.0:0)

use mycnet_core::mesh::FractalMeshPacket;
use mycnet_core::s60::HolographicSeedBinary;
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::net::UdpSocket;
use std::time::Duration;

#[derive(Deserialize)]
struct WalEnvelope {
    event_id: String,
    #[serde(default)]
    payload: String,
}

// payload es JSON con {data: "<json>", event_type, source};
// data es JSON con {entropy_s60_raw, guardian_event_type, pid, severity, ...}.
#[derive(Deserialize)]
struct WalOuter {
    #[serde(default)]
    data: String,
}

#[derive(Deserialize)]
struct WalInner {
    #[serde(default)]
    entropy_s60_raw: i64,
    #[serde(default)]
    severity: u8,
    #[serde(default)]
    guardian_code: u8,
    #[serde(default)]
    pid: u32,
}

fn token_of(event_id: &str) -> u32 {
    let mut h = DefaultHasher::new();
    event_id.hash(&mut h);
    (h.finish() & 0xFFFF_FFFF) as u32
}

/// Los 37 nodos de la malla hexagonal de 3 anillos, en espiral axial.
/// Debe coincidir con `mycnet_hex_coordinate` de sentinel-cortex.
fn hex_of(index: usize) -> (i32, i32) {
    // Anillo 0: centro. Anillos 1-3: 6*k celdas en espiral axial.
    if index == 0 {
        return (0, 0);
    }
    let dirs = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];
    let mut idx = index;
    let mut ring = 1;
    loop {
        let count = 6 * ring;
        if idx <= count {
            let side = (idx - 1) / ring;
            let step = (idx - 1) % ring;
            let (dq, dr) = dirs[side];
            // Punto inicial del lado: ring * dirs[(side+4)%6]
            let (sq, sr) = dirs[(side + 4) % 6];
            return (ring as i32 * sq + step as i32 * dq, ring as i32 * sr + step as i32 * dr);
        }
        idx -= count;
        ring += 1;
    }
}

fn seed_of(env: &WalEnvelope) -> Option<HolographicSeedBinary> {
    let outer: WalOuter = serde_json::from_str(&env.payload).ok()?;
    let inner: WalInner = serde_json::from_str(&outer.data).ok()?;
    let (q, r) = hex_of((inner.pid as usize) % 37);
    Some(HolographicSeedBinary::new(
        token_of(&env.event_id),
        q,
        r,
        -q - r,
        inner.entropy_s60_raw,
        inner.severity % 60,
        inner.guardian_code,
        0x01,
    ))
}

fn main() -> anyhow::Result<()> {
    let wal = std::env::var("GUARDIAN_WAL")
        .unwrap_or_else(|_| "/var/log/sentinel/guardian_security_wal.jsonl".into());
    let peer: std::net::SocketAddr = std::env::var("MYCNET_UPLINK_PEER")
        .unwrap_or_else(|_| "10.88.0.1:4242".into())
        .parse()?;
    let bind: std::net::SocketAddr = std::env::var("MYCNET_UPLINK_BIND")
        .unwrap_or_else(|_| "0.0.0.0:0".into())
        .parse()?;
    let sock = UdpSocket::bind(bind)?;
    sock.set_write_timeout(Some(Duration::from_secs(2)))?;

    let mut seq: u16 = 0;
    let mut file = std::fs::File::open(&wal)?;
    // Arrancar en la cola: solo eventos nuevos cantan (el pasado ya esta en el WAL).
    file.seek(SeekFrom::End(0))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();

    tracing_subscriber::fmt().with_env_filter("info").init();
    tracing::info!(wal = %wal, peer = %peer, "guardian-uplink activo: WAL → mesh");

    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            std::thread::sleep(Duration::from_millis(200));
            continue;
        }
        let env: WalEnvelope = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let Some(seed) = seed_of(&env) else { continue };
        seq = seq.wrapping_add(1);
        let pkt = FractalMeshPacket::new(seq, seed);
        if let Err(e) = sock.send_to(&pkt.to_bytes(), peer) {
            tracing::warn!(error = %e, "fallo envio mesh");
        }
    }
}
