// Autor: Jaime Novoa Sepúlveda — Todos los derechos reservados.
//! guardian-uplink — Adaptador WAL Guardian → mesh MyCNet (Marte → Fenix).
//!
//! Lee el WAL del security-gateway (`guardian_security_wal.jsonl`) en tail,
//! convierte cada evento eBPF a `HolographicSeedBinary` y lo envía por UDP
//! al peer MyCNet (Fenix), dentro de la red privada.
//!
//! Proyección del evento al formato de transporte (no mide fase del cristal):
//! - token_id: hash del event_id (u32).
//! - coord (q,r,s): hexágono del índice = pid % 37 nodos.
//! - amplitude_s60: entropy_s60_raw del evento, sin volver a escalar.
//! - phase_sector: severity % 60; yhwh_pulse: guardian_code; flags: 0x01.
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
    payload: String,
}

// payload es texto JSON; su campo data es un objeto, no otro texto JSON.
// El código del Guardian está dentro de data.sanitized_event.
#[derive(Deserialize)]
struct WalOuter {
    data: WalInner,
}

#[derive(Deserialize)]
struct WalInner {
    entropy_s60_raw: i64,
    severity: u8,
    pid: u32,
    sanitized_event: SanitizedEvent,
}

#[derive(Deserialize)]
struct SanitizedEvent {
    guardian_code: u8,
}

fn token_of(event_id: &str) -> u32 {
    let mut h = DefaultHasher::new();
    event_id.hash(&mut h);
    (h.finish() & 0xFFFF_FFFF) as u32
}

/// Los 37 nodos del centro y los tres primeros anillos en coordenadas axiales.
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

fn seed_of(env: &WalEnvelope) -> Result<HolographicSeedBinary, serde_json::Error> {
    let outer: WalOuter = serde_json::from_str(&env.payload)?;
    let inner = outer.data;
    let (q, r) = hex_of((inner.pid as usize) % 37);
    Ok(HolographicSeedBinary::new(
        token_of(&env.event_id),
        q,
        r,
        -q - r,
        inner.entropy_s60_raw,
        inner.severity % 60,
        inner.sanitized_event.guardian_code,
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
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            std::thread::sleep(Duration::from_millis(200));
            continue;
        }
        // Un append puede ser visible antes de completar la línea del WAL.
        if !line.ends_with('\n') {
            continue;
        }
        let parsed = serde_json::from_str::<WalEnvelope>(&line);
        line.clear();
        let env = match parsed {
            Ok(e) => e,
            Err(error) => {
                tracing::warn!(%error, "registro WAL inválido; no se envía");
                continue;
            }
        };
        let seed = match seed_of(&env) {
            Ok(seed) => seed,
            Err(error) => {
                tracing::warn!(event_id = %env.event_id, %error, "payload Guardian inválido; no se envía");
                continue;
            }
        };
        seq = seq.wrapping_add(1);
        let pkt = FractalMeshPacket::new(seq, seed);
        if let Err(e) = sock.send_to(&pkt.to_bytes(), peer) {
            tracing::warn!(error = %e, "fallo envio mesh");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(amplitude: i64) -> WalEnvelope {
        WalEnvelope {
            event_id: "guardian_alpha-80326144118329-223991-EXEC_ALLOWED".into(),
            payload: serde_json::json!({
                "event_type": "syscall",
                "source": "guardian_alpha",
                "data": {
                    "entropy_s60_raw": amplitude,
                    "guardian_event_type": "EXEC_ALLOWED",
                    "pid": 223991,
                    "severity": 1,
                    "sanitized_event": { "guardian_code": 2 }
                }
            }).to_string(),
        }
    }

    #[test]
    fn wal_object_preserves_raw_amplitude_and_guardian_through_wire() {
        for amplitude in [0, 12_960_000, -1, i64::MIN, i64::MAX] {
            let seed = seed_of(&envelope(amplitude)).unwrap();
            let bytes = FractalMeshPacket::new(7, seed).to_bytes();
            let decoded = FractalMeshPacket::from_bytes(&bytes).unwrap();
            assert_eq!(decoded.seed.amplitude_s60, amplitude);
            assert_eq!(decoded.seed.yhwh_pulse, 2);
            assert_eq!(decoded.seed.phase_sector, 1);
            assert_eq!(
                decoded.seed.coord_q + decoded.seed.coord_r + decoded.seed.coord_s,
                0
            );
        }
    }

    #[test]
    fn malformed_payload_cannot_become_a_zero_seed() {
        let mut env = envelope(12_960_000);
        for payload in [
            r#"{"data":"{}"}"#,
            r#"{"data":{"pid":1,"severity":1,"sanitized_event":{"guardian_code":2}}}"#,
            r#"{"data":{"pid":1,"severity":1,"entropy_s60_raw":0}}"#,
            r#"{"data":{"pid":1,"severity":1,"entropy_s60_raw":9223372036854775808,"sanitized_event":{"guardian_code":2}}}"#,
        ] {
            env.payload = payload.into();
            assert!(seed_of(&env).is_err());
        }
    }
}
