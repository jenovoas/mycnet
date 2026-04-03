//! # ADM — Axial Diffusion Model
//!
//! Red resonante micelial hexagonal bio-inspirada.
//! Salto-17: distribución de fases soberana en coordenadas axiales S60.
//!
//! Portado y extendido desde sentinel-cubepath/backend/src/mycnet.rs
//! Autor: Jaime Novoa — Yatra Protocol

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::s60::SPA;

/// Coordenadas axiales hexagonales expresadas en grados S60.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AxialCoord {
    pub q: SPA,
    pub r: SPA,
}

impl AxialCoord {
    pub fn new(q: i64, r: i64) -> Self {
        Self {
            q: SPA::new(q, 0, 0, 0, 0),
            r: SPA::new(r, 0, 0, 0, 0),
        }
    }

    /// Los 6 vecinos hexagonales en coordenadas axiales.
    pub fn neighbors(&self) -> [AxialCoord; 6] {
        let one = SPA::one();
        [
            AxialCoord { q: self.q + one,  r: self.r         },
            AxialCoord { q: self.q + one,  r: self.r - one   },
            AxialCoord { q: self.q,        r: self.r - one   },
            AxialCoord { q: self.q - one,  r: self.r         },
            AxialCoord { q: self.q - one,  r: self.r + one   },
            AxialCoord { q: self.q,        r: self.r + one   },
        ]
    }
}

/// Nodo micelial — unidad de procesamiento del ADM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MycNode {
    pub coord:     AxialCoord,
    /// Salud energética del nodo (TQ normalizado a SPA)
    pub amplitude: SPA,
    /// Sincronización de fase en grados S60 (Salto-17)
    pub phase_s60: SPA,
    /// ID de nodo físico asociado (MAC o hostname), si existe
    pub physical_id: Option<String>,
}

/// Axial Diffusion Model — motor de propagación bio-inspirado.
///
/// Reglas:
/// 1. Nodos sobre umbral (1;15;0;0;0) difunden energía a sus 6 vecinos.
/// 2. Fase inicial: (índice × 17) % 60 grados — Salto-17 soberano.
/// 3. Un tick = un ciclo de propagación completo.
pub struct ADM {
    pub nodes:    HashMap<AxialCoord, MycNode>,
    /// Salto axiomático de fase = 17
    step_key:     SPA,
    /// Número de ticks ejecutados
    pub tick_count: u64,
}

impl ADM {
    const SPIKE_THRESHOLD: SPA = SPA::new(1, 15, 0, 0, 0);
    const RECHARGE_STATE:  SPA = SPA::new(0, 30, 0, 0, 0);

    pub fn new() -> Self {
        Self {
            nodes:      HashMap::new(),
            step_key:   SPA::from_int(17),
            tick_count: 0,
        }
    }

    /// Agrega un nodo virtual en posición axial (q, r).
    pub fn add_node(&mut self, q: i64, r: i64) {
        let coord = AxialCoord::new(q, r);
        let n = self.nodes.len() as i64;
        let phase_deg = (n * 17) % 60;
        self.nodes.insert(coord, MycNode {
            coord,
            amplitude:   SPA::one(),
            phase_s60:   SPA::new(phase_deg, 0, 0, 0, 0),
            physical_id: None,
        });
    }

    /// Vincula un nodo virtual a un nodo físico de la red batman-adv.
    pub fn bind_physical(&mut self, q: i64, r: i64, physical_id: String) {
        if let Some(node) = self.nodes.get_mut(&AxialCoord::new(q, r)) {
            node.physical_id = Some(physical_id);
        }
    }

    /// Inyecta energía (TQ del nodo físico) en un nodo del ADM.
    pub fn inject_tq(&mut self, q: i64, r: i64, tq: u8) {
        let energy = SPA::from_tq(tq);
        if let Some(node) = self.nodes.get_mut(&AxialCoord::new(q, r)) {
            node.amplitude = energy;
        }
    }

    /// Ciclo de propagación bio-inspirada.
    /// Llamar una vez por tick del loop isocrono.
    pub fn tick(&mut self) {
        let mut spikes: Vec<(AxialCoord, SPA)> = Vec::new();

        for (coord, node) in &mut self.nodes {
            if node.amplitude > Self::SPIKE_THRESHOLD {
                let energy_per_neighbor = node.amplitude
                    .div_safe(SPA::from_int(12))
                    .unwrap_or(SPA::zero());
                for neighbor in coord.neighbors() {
                    spikes.push((neighbor, energy_per_neighbor));
                }
                node.amplitude = Self::RECHARGE_STATE;
            }
        }

        for (coord, energy) in spikes {
            if let Some(target) = self.nodes.get_mut(&coord) {
                target.amplitude = target.amplitude + energy;
            }
        }

        self.tick_count += 1;
    }

    /// Coherencia global: promedio de amplitudes normalizadas.
    pub fn coherence(&self) -> SPA {
        if self.nodes.is_empty() { return SPA::zero(); }
        let sum: i64 = self.nodes.values()
            .map(|n| n.amplitude.to_raw())
            .sum();
        SPA::from_raw(sum / self.nodes.len() as i64)
    }

    /// Snapshot de todos los nodos (para API/WebSocket).
    pub fn snapshot(&self) -> Vec<NodeSnapshot> {
        self.nodes.values().map(|n| NodeSnapshot {
            q:           n.coord.q.to_degrees(),
            r:           n.coord.r.to_degrees(),
            amplitude:   n.amplitude.to_string(),
            phase_s60:   n.phase_s60.to_string(),
            physical_id: n.physical_id.clone(),
        }).collect()
    }
}

impl Default for ADM {
    fn default() -> Self { Self::new() }
}

/// Vista serializable de un nodo para la API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSnapshot {
    pub q:           i64,
    pub r:           i64,
    pub amplitude:   String,
    pub phase_s60:   String,
    pub physical_id: Option<String>,
}
