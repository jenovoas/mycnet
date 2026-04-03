//! # mesh — Integración batman-adv real
//!
//! Lee el estado físico de la red desde `batctl` y lo expone
//! como estructuras tipadas con métricas TQ en SPA.
//!
//! Autor: Jaime Novoa — Yatra Protocol

use std::process::Command;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::s60::SPA;

#[derive(Debug, Error)]
pub enum MeshError {
    #[error("batctl no disponible: {0}")]
    BatctlNotFound(String),
    #[error("Error al parsear salida de batctl: {0}")]
    ParseError(String),
    #[error("Error de IO: {0}")]
    IoError(#[from] std::io::Error),
}

/// Vecino detectado por batman-adv (salida de `batctl n`).
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
    pub nodes:    Vec<PhysicalNode>,
    pub ts_unix:  u64,
}

/// Parsea la salida de `batctl n` (neighbors).
///
/// Formato típico:
/// ```
/// [B.A.T.M.A.N. adv 2023.3, MainIF/MAC: eth1/aa:bb:cc:dd:ee:ff (bat0 BATMAN_IV)]
/// Neighbor         last-seen Quali Iface
/// aa:bb:cc:dd:00:01    0.200s  250 eth1
/// aa:bb:cc:dd:00:02    0.500s  180 eth1
/// ```
pub fn parse_batctl_neighbors(output: &str) -> Result<Vec<Neighbor>, MeshError> {
    let mut neighbors = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Línea de datos: MAC  tiempo  TQ  iface
        // MAC tiene formato xx:xx:xx:xx:xx:xx (17 chars)
        if parts.len() >= 4 && parts[0].len() == 17 && parts[0].contains(':') {
            let mac = parts[0].to_string();
            let iface = parts[3].to_string();

            // last-seen: "0.200s" → ms
            let last_seen_ms = parse_last_seen(parts[1]);

            // TQ puede ser el tercer campo
            let tq: u8 = parts[2].parse().map_err(|_| {
                MeshError::ParseError(format!("TQ inválido: {}", parts[2]))
            })?;

            neighbors.push(Neighbor {
                tq_s60: SPA::from_tq(tq),
                mac,
                iface,
                tq,
                last_seen_ms,
            });
        }
    }

    Ok(neighbors)
}

/// Ejecuta `batctl n` y retorna los vecinos del nodo local.
pub fn get_local_neighbors() -> Result<Vec<Neighbor>, MeshError> {
    let output = Command::new("batctl")
        .arg("n")
        .output()
        .map_err(|e| MeshError::BatctlNotFound(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_batctl_neighbors(&stdout)
}

/// Calcula la coherencia media de la red como SPA.
pub fn mesh_coherence(neighbors: &[Neighbor]) -> SPA {
    if neighbors.is_empty() { return SPA::zero(); }
    let sum: i64 = neighbors.iter().map(|n| n.tq_s60.to_raw()).sum();
    SPA::from_raw(sum / neighbors.len() as i64)
}

fn parse_last_seen(s: &str) -> u64 {
    // "0.200s" → 200ms
    let s = s.trim_end_matches('s');
    s.parse::<f64>()
        .map(|v| (v * 1000.0) as u64)
        .unwrap_or(0)
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

    #[test]
    fn parse_three_neighbors() {
        let neighbors = parse_batctl_neighbors(SAMPLE_OUTPUT).unwrap();
        assert_eq!(neighbors.len(), 3);
        assert_eq!(neighbors[0].tq, 250);
        assert_eq!(neighbors[1].tq, 180);
        assert_eq!(neighbors[2].tq, 90);
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
}
