// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! # mesh — Integración batman-adv real
//!
//! Lee el estado físico de la red desde `batctl` y lo expone
//! como estructuras tipadas con métricas TQ en SPA.
//!
//! Autor: Jaime Novoa — Yatra Protocol

use crate::s60::{HolographicSeedBinary, SPA};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub const FRACTAL_PACKET_MAGIC: u32 = 0x4D594353; // "MYCS"
pub const FRACTAL_PROTOCOL_VERSION: u16 = 1;

/// Paquete de red para transporte de semillas fractales sobre malla (40 bytes exactos).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FractalMeshPacket {
    pub magic: u32,
    pub version: u16,
    pub sequence_id: u16,
    pub seed: HolographicSeedBinary,
}

const _: () = assert!(
    std::mem::size_of::<FractalMeshPacket>() == 40,
    "FractalMeshPacket debe medir exactamente 40 bytes según OpenSpec"
);

impl FractalMeshPacket {
    pub fn new(sequence_id: u16, seed: HolographicSeedBinary) -> Self {
        Self {
            magic: FRACTAL_PACKET_MAGIC,
            version: FRACTAL_PROTOCOL_VERSION,
            sequence_id,
            seed,
        }
    }

    pub fn to_bytes(&self) -> [u8; 40] {
        let mut bytes = [0u8; 40];
        bytes[0..4].copy_from_slice(&self.magic.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.version.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.sequence_id.to_le_bytes());
        bytes[8..40].copy_from_slice(&self.seed.to_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8; 40]) -> Result<Self, &'static str> {
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != FRACTAL_PACKET_MAGIC {
            return Err("Magic header inválido (no es un paquete MYCS)");
        }
        let version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        if version != FRACTAL_PROTOCOL_VERSION {
            return Err("Versión de protocolo de semilla no soportada");
        }
        let sequence_id = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
        let seed_bytes: [u8; 32] = bytes[8..40].try_into().unwrap();
        let seed = HolographicSeedBinary::from_bytes(&seed_bytes)?;

        Ok(Self {
            magic,
            version,
            sequence_id,
            seed,
        })
    }
}

#[derive(Debug, Error)]
pub enum MeshError {
    #[error("batctl no disponible: {0}")]
    BatctlNotFound(String),
    #[error("Error al parsear salida de batctl: {0}")]
    ParseError(String),
    #[error("Error de IO: {0}")]
    IoError(#[from] std::io::Error),
}

/// Vecino directo detectado por `batctl n`, con TQ de la ruta en `batctl o`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neighbor {
    /// Dirección MAC del vecino
    pub mac: String,
    /// Interfaz física por donde se ve
    pub iface: String,
    /// Transmit Quality (0-255)
    pub tq: u8,
    /// TQ convertido a SPA normalizado [0,1)
    pub tq_s60: SPA,
    /// Tiempo desde último anuncio (ms)
    pub last_seen_ms: u64,
}

/// Nodo físico completo con sus vecinos y métricas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalNode {
    /// Hostname o identificador del nodo
    pub id: String,
    /// IP en la red bat0
    pub bat_ip: String,
    /// Lista de vecinos con TQ
    pub neighbors: Vec<Neighbor>,
    /// Coherencia media S60 de todos los vecinos
    pub mesh_coherence: SPA,
}

/// Estado global de la red mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshState {
    pub nodes: Vec<PhysicalNode>,
    pub ts_unix: u64,
}

/// Parsea la salida de `batctl n` (neighbors).
///
/// Formato típico:
/// ```text
/// [B.A.T.M.A.N. adv 2023.3, MainIF/MAC: eth1/aa:bb:cc:dd:ee:ff (bat0 BATMAN_IV)]
/// Neighbor         last-seen Quali Iface
/// aa:bb:cc:dd:00:01    0.200s  250 eth1
/// aa:bb:cc:dd:00:02    0.500s  180 eth1
/// ```
pub fn parse_batctl_neighbors(output: &str) -> Result<Vec<Neighbor>, MeshError> {
    let mut neighbors = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Formato histórico: MAC tiempo TQ interfaz.
        if parts.len() >= 4 && is_mac_address(parts[0]) {
            let mac = parts[0].to_string();
            let iface = parts[3].to_string();
            let last_seen_ms = parse_last_seen(parts[1]);
            let tq = parts[2]
                .parse::<u8>()
                .map_err(|_| MeshError::ParseError(format!("TQ inválido: {}", parts[2])))?;

            neighbors.push(Neighbor {
                tq_s60: SPA::from_tq(tq),
                mac,
                iface,
                tq,
                last_seen_ms,
            });
        // Formato actual de batctl n: interfaz MAC tiempo. El TQ se completa
        // desde la ruta preferida de ese mismo origen en la tabla `batctl o`.
        } else if parts.len() >= 3 && is_mac_address(parts[1]) {
            neighbors.push(Neighbor {
                mac: parts[1].to_string(),
                iface: parts[0].to_string(),
                tq: 0,
                tq_s60: SPA::zero(),
                last_seen_ms: parse_last_seen(parts[2]),
            });
        }
    }

    Ok(neighbors)
}

fn is_mac_address(value: &str) -> bool {
    value.len() == 17 && value.bytes().filter(|byte| *byte == b':').count() == 5
}

/// Une vecinos directos de `batctl n` con sus métricas TQ de `batctl o`.
pub fn parse_batctl_snapshot(output: &str) -> Result<Vec<Neighbor>, MeshError> {
    let (neighbor_output, originator_output) = output
        .split_once("---MYCNET-ORIGINATORS---")
        .ok_or_else(|| MeshError::ParseError("instantánea Batman incompleta".into()))?;
    let mut neighbors = parse_batctl_neighbors(neighbor_output)?;
    let originators = parse_batctl_originators(originator_output)?;

    for neighbor in &mut neighbors {
        if let Some(originator) = originators.iter().find(|item| item.mac == neighbor.mac) {
            neighbor.tq = originator.tq;
            neighbor.tq_s60 = originator.tq_s60;
        }
    }
    Ok(neighbors)
}

/// Parsea la tabla de originadores de `batctl o`.
///
/// batctl moderno muestra cada ruta como `MAC tiempo (TQ) next-hop [interfaz]`;
/// puede haber varias rutas por origen. Se conserva la marcada como preferida
/// (`*`) y se usa TQ como desempate si la salida no marca una ruta.
pub fn parse_batctl_originators(output: &str) -> Result<Vec<Neighbor>, MeshError> {
    let mut selected: BTreeMap<String, (bool, Neighbor)> = BTreeMap::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let Some(mac_index) = parts.iter().position(|part| {
            part.len() == 17 && part.bytes().filter(|byte| *byte == b':').count() == 5
        }) else {
            continue;
        };
        let Some(last_seen) = parts.get(mac_index + 1) else {
            continue;
        };
        let Some(tq_field) = parts.get(mac_index + 2) else {
            continue;
        };
        let Some(tq_text) = tq_field.trim_start_matches('(').strip_suffix(')') else {
            continue;
        };
        let tq = tq_text
            .parse::<u8>()
            .map_err(|_| MeshError::ParseError(format!("TQ inválido: {}", tq_text)))?;
        let Some(iface) = parts.last().map(|part| part.trim_end_matches(']')) else {
            continue;
        };
        if iface.is_empty() || iface == "[" {
            continue;
        }

        let mac = parts[mac_index].to_string();
        let preferred = parts.first() == Some(&"*");
        let neighbor = Neighbor {
            mac: mac.clone(),
            iface: iface.to_string(),
            tq,
            tq_s60: SPA::from_tq(tq),
            last_seen_ms: parse_last_seen(last_seen),
        };

        let replace = selected
            .get(&mac)
            .map(|(old_preferred, old)| {
                (preferred && !old_preferred) || (preferred == *old_preferred && tq > old.tq)
            })
            .unwrap_or(true);
        if replace {
            selected.insert(mac, (preferred, neighbor));
        }
    }

    Ok(selected
        .into_values()
        .map(|(_, neighbor)| neighbor)
        .collect())
}

/// Lee la instantánea periódica de `batctl n/o` y retorna los vecinos directos
/// con su TQ. El sondeo privilegiado corre aparte; el daemon MycNet no necesita
/// capacidades de administración de red.
pub async fn get_local_neighbors() -> Result<Vec<Neighbor>, MeshError> {
    let snapshot = std::fs::read_to_string("/run/mycnet/batctl-originators")?;
    parse_batctl_snapshot(&snapshot)
}

/// Calcula la coherencia media de la red como SPA.
pub fn mesh_coherence(neighbors: &[Neighbor]) -> SPA {
    if neighbors.is_empty() {
        return SPA::zero();
    }
    let sum: i64 = neighbors.iter().map(|n| n.tq_s60.to_raw()).sum();
    SPA::from_raw(sum / neighbors.len() as i64)
}

fn parse_last_seen(s: &str) -> u64 {
    // "0.200s" → 200 ms (S60 puro, sin f64).
    // Formato batctl: d.ddd s — separar parte entera y decimal de segundos.
    // 1 s = 1000 ms, 1 s = 60^2 = 3600 tercias S60.
    // Conversion: ms = (int_secs * 1000 + frac_int * 1000 / 10^decimales)
    // Scale-up primero, dividir al final para evitar truncamiento temprano.
    let s = s.trim_end_matches('s');
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s, ""),
    };

    let int_secs: u64 = int_part.parse().unwrap_or(0);

    // Fracción decimal: 0.200 → 200 ms (×1000/10^3)
    let frac_ms: u64 = if frac_part.is_empty() {
        0
    } else {
        let frac_int: u64 = frac_part.parse().unwrap_or(0);
        let denom: u64 = 10u64.pow(frac_part.len() as u32);
        (frac_int * 1000) / denom
    };

    int_secs * 1000 + frac_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OUTPUT: &str = r#"
[B.A.T.M.A.N. adv 2023.3, MainIF/MAC: eth1/aa:00:00:00:00:01 (bat0 BATMAN_IV)]
Neighbor         last-seen Quali Iface
aa:00:00:00:00:02    0.200s  250 eth1
aa:00:00:00:00:03    0.500s  180 eth1
aa:00:00:00:00:04    1.000s   90 eth1
"#;

    const SAMPLE_ORIGINATORS: &str = r#"
[B.A.T.M.A.N. adv 2024.2, MainIF/MAC: vxmc1/aa:00:00:00:00:01 (bat0 BATMAN_IV)]
   Originator        last-seen (#/255) Nexthop           [outgoingIF]
   aa:00:00:00:00:02    0.200s   (218) aa:00:00:00:00:03 [     vxmc2]
 * aa:00:00:00:00:02    0.200s   (250) aa:00:00:00:00:02 [     vxmc3]
 * aa:00:00:00:00:03    0.500s   (180) aa:00:00:00:00:03 [     vxmc4]
"#;

    const SAMPLE_SNAPSHOT: &str = r#"
[B.A.T.M.A.N. adv 2024.2, MainIF/MAC: vxmc1/aa:00:00:00:00:01 (bat0/aa:00:00:00:00:11 BATMAN_IV)]
IF             Neighbor              last-seen
vxmc2          aa:00:00:00:00:02    0.200s
vxmc3          aa:00:00:00:00:03    0.500s
---MYCNET-ORIGINATORS---
[B.A.T.M.A.N. adv 2024.2, MainIF/MAC: vxmc1/aa:00:00:00:00:01 (bat0/aa:00:00:00:00:11 BATMAN_IV)]
 * aa:00:00:00:00:02    0.200s   (250) aa:00:00:00:00:02 [     vxmc2]
 * aa:00:00:00:00:03    0.500s   (180) aa:00:00:00:00:03 [     vxmc3]
"#;

    #[test]
    fn parse_three_neighbors() {
        let neighbors = parse_batctl_neighbors(SAMPLE_OUTPUT).unwrap();
        assert_eq!(neighbors.len(), 3);
        assert_eq!(neighbors[0].tq, 250);
        assert_eq!(neighbors[1].tq, 180);
        assert_eq!(neighbors[2].tq, 90);
    }

    #[test]
    fn parse_current_batctl_originators_and_preferred_route() {
        let neighbors = parse_batctl_originators(SAMPLE_ORIGINATORS).unwrap();
        assert_eq!(neighbors.len(), 2);
        assert_eq!(neighbors[0].mac, "aa:00:00:00:00:02");
        assert_eq!(neighbors[0].tq, 250);
        assert_eq!(neighbors[0].iface, "vxmc3");
        assert_eq!(neighbors[0].last_seen_ms, 200);
        assert_eq!(neighbors[1].tq, 180);
    }

    #[test]
    fn parse_batctl_snapshot_keeps_direct_peers_and_adds_tq() {
        let neighbors = parse_batctl_snapshot(SAMPLE_SNAPSHOT).unwrap();
        assert_eq!(neighbors.len(), 2);
        assert_eq!(neighbors[0].mac, "aa:00:00:00:00:02");
        assert_eq!(neighbors[0].iface, "vxmc2");
        assert_eq!(neighbors[0].tq, 250);
        assert_eq!(neighbors[1].tq, 180);
    }

    #[test]
    fn tq_255_is_spa_one() {
        let n = Neighbor {
            mac: "aa:00:00:00:00:01".into(),
            iface: "eth1".into(),
            tq: 255,
            tq_s60: SPA::from_tq(255),
            last_seen_ms: 100,
        };
        assert_eq!(n.tq_s60, SPA::one());
    }

    #[test]
    fn fractal_mesh_packet_wire_layout_40_bytes() {
        assert_eq!(std::mem::size_of::<FractalMeshPacket>(), 40);
    }

    #[test]
    fn fractal_mesh_packet_roundtrip_and_corruption_rejection() {
        let seed = HolographicSeedBinary::from_lfm(123, 2, -2, 45);
        let packet = FractalMeshPacket::new(1, seed);

        let bytes = packet.to_bytes();
        assert_eq!(bytes.len(), 40);

        let decoded = FractalMeshPacket::from_bytes(&bytes).expect("Decodificación falló");
        assert_eq!(packet, decoded);

        let mut bad_magic = bytes;
        bad_magic[0] = 0x00;
        assert!(FractalMeshPacket::from_bytes(&bad_magic).is_err());

        let mut bad_payload = bytes;
        bad_payload[10] ^= 0xFF;
        assert!(FractalMeshPacket::from_bytes(&bad_payload).is_err());
    }
}
