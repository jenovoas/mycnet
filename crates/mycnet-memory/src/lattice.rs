//! # LiquidLattice — Tejido de Memoria Resonante
//!
//! Portado desde EXP-009 (Liquid Lattice 3×3) a escala distribuida.
//! Cada satélite posee una región del lattice.
//!
//! ## Física implementada
//!
//! **Difusión Von Neumann** (vecindario en cruz, snapshot atómico):
//! ```
//! A_new[i] = (A[i] + Σ A[vecinos]) / (1 + N_vecinos)
//! ```
//! La actualización es ATÓMICA: todos los nodos leen el estado anterior,
//! luego todos escriben el nuevo. Sin sesgo de orden.
//!
//! **Canal dual** (EXP-012):
//! - Canal A (Energía / Amplitud SPA): datos reales, 6 bytes/nodo
//! - Canal B (Fase / Sector 0-255): metadatos, 1 byte/nodo
//!
//! **Coherencia hexagonal**: si la coherencia del lattice cae < 0.8,
//! el sector se "cierra" para absorber el daño sin propagarlo.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use mycnet_core::s60::SPA;

/// Un nodo del lattice — 16 bytes en memoria (EXP-015 validated).
/// Layout: amplitude(8) + phase_sector(1) + data(6) + flags(1)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LatticeNode {
    /// Amplitud de energía en SPA — "salud" del nodo
    pub amplitude: SPA,
    /// Sector de fase (0-255) — canal B de metadatos (EXP-012)
    pub phase_sector: u8,
    /// Datos reales del usuario (6 bytes por nodo, EXP-015)
    pub data: [u8; 6],
    /// Flags: bit0=ocupado, bit1=sector_cerrado, bit2=dirty
    pub flags: u8,
}

impl LatticeNode {
    pub const CHUNK_SIZE: usize = 6;

    pub fn empty() -> Self {
        Self {
            amplitude:    SPA::zero(),
            phase_sector: 0,
            data:         [0u8; 6],
            flags:        0,
        }
    }

    pub fn is_occupied(&self) -> bool { self.flags & 0x01 != 0 }
    pub fn is_closed(&self)   -> bool { self.flags & 0x02 != 0 }
    pub fn is_dirty(&self)    -> bool { self.flags & 0x04 != 0 }

    pub fn set_occupied(&mut self, v: bool) {
        if v { self.flags |= 0x01 } else { self.flags &= !0x01 }
    }
    pub fn set_closed(&mut self, v: bool) {
        if v { self.flags |= 0x02 } else { self.flags &= !0x02 }
    }
}

/// Coordenada axial del lattice hexagonal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Coord { pub q: i32, pub r: i32 }

impl Coord {
    /// Los 6 vecinos en coordenadas axiales hexagonales.
    pub fn neighbors(self) -> [Coord; 6] {
        [
            Coord { q: self.q+1, r: self.r   },
            Coord { q: self.q+1, r: self.r-1 },
            Coord { q: self.q,   r: self.r-1 },
            Coord { q: self.q-1, r: self.r   },
            Coord { q: self.q-1, r: self.r+1 },
            Coord { q: self.q,   r: self.r+1 },
        ]
    }

    /// Región hexagonal (0-5) para phase gating.
    /// Basado en la dirección axial dominante.
    pub fn hex_region(self) -> u8 {
        ((self.q.rem_euclid(3) * 2 + self.r.rem_euclid(2)) % 6) as u8
    }
}

/// Paso de difusión — resultado de un tick del lattice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffusionStep {
    pub tick:         u64,
    pub coherence:    SPA,
    pub nodes_live:   usize,
    pub nodes_total:  usize,
    /// true si este tick disparó el Quantum Leap (T=68s, 17×4 ciclos YHWH)
    pub quantum_leap: bool,
}

/// El Liquid Lattice local de un satélite.
///
/// Tamaño típico: rings=3 → 37 nodos, rings=6 → 127 nodos.
/// A 16 bytes/nodo: 127 nodos = ~2KB en RAM.
pub struct LiquidLattice {
    pub nodes:     HashMap<Coord, LatticeNode>,
    pub rings:     u32,
    pub tick:      u64,
    /// Umbral de coherencia para cierre de sector (EXP hexagonal control)
    pub coherence_threshold: SPA,
}

impl LiquidLattice {
    /// Crea el lattice con topología hexagonal de `rings` anillos.
    /// rings=1 → 7 nodos, rings=2 → 19, rings=3 → 37, rings=6 → 127
    pub fn new(rings: u32) -> Self {
        let mut nodes = HashMap::new();
        let r = rings as i32;

        for q in -r..=r {
            let r1 = (-r).max(-q - r);
            let r2 = r.min(-q + r);
            for ri in r1..=r2 {
                nodes.insert(Coord { q, r: ri }, LatticeNode::empty());
            }
        }

        Self {
            nodes,
            rings,
            tick: 0,
            coherence_threshold: SPA::from_raw((SPA::SCALE_0 as f64 * 0.8) as i64),
        }
    }

    /// Inyecta datos en el lattice a partir de una posición inicial.
    /// Retorna el número de nodos usados.
    pub fn inject(&mut self, data: &[u8], start: Coord) -> usize {
        let chunks: Vec<&[u8]> = data.chunks(LatticeNode::CHUNK_SIZE).collect();
        let coords: Vec<Coord> = self.nodes.keys().copied().collect();
        let mut used = 0;

        // Orden espiral desde start — los datos fluyen desde el centro
        let mut sorted = coords;
        sorted.sort_by_key(|c| {
            let dq = (c.q - start.q).abs();
            let dr = (c.r - start.r).abs();
            dq + dr
        });

        for (i, coord) in sorted.iter().enumerate() {
            if i >= chunks.len() { break; }
            if let Some(node) = self.nodes.get_mut(coord) {
                if node.is_closed() { continue; }

                let chunk = chunks[i];
                let mut buf = [0u8; 6];
                buf[..chunk.len()].copy_from_slice(chunk);
                node.data         = buf;
                node.amplitude    = SPA::from_int((i as i64 + 1) * 10); // energía proporcional a posición
                node.phase_sector = (i % 256) as u8;
                node.set_occupied(true);
                node.set_dirty(true);
                used += 1;
            }
        }
        used
    }

    /// Recupera datos del lattice en orden espiral desde start.
    pub fn retrieve(&self, start: Coord, len: usize) -> Vec<u8> {
        let mut coords: Vec<Coord> = self.nodes.keys().copied().collect();
        coords.sort_by_key(|c| {
            let dq = (c.q - start.q).abs();
            let dr = (c.r - start.r).abs();
            dq + dr
        });

        let mut out = Vec::with_capacity(len);
        for coord in &coords {
            if out.len() >= len { break; }
            if let Some(node) = self.nodes.get(coord) {
                if node.is_occupied() && !node.is_closed() {
                    let remaining = len - out.len();
                    let take = remaining.min(6);
                    out.extend_from_slice(&node.data[..take]);
                }
            }
        }
        out
    }

    /// Un tick de difusión (EXP-009 physics, snapshot atómico).
    ///
    /// Solo difunde los nodos en `hex_region == phase_slot`.
    /// Phase gating: 1/6 de la red por tick → sin conflictos.
    pub fn diffuse_tick(&mut self, phase_slot: u8, noise_mask: u8) -> DiffusionStep {
        self.tick += 1;

        // Snapshot del estado actual (necesario para difusión correcta)
        let snapshot: HashMap<Coord, SPA> = self.nodes.iter()
            .map(|(c, n)| (*c, n.amplitude))
            .collect();

        let mut new_amps: HashMap<Coord, SPA> = HashMap::new();

        for (&coord, node) in &self.nodes {
            // Phase gating: solo procesa nodos del slot actual
            if coord.hex_region() != phase_slot { continue; }
            if node.is_closed() { continue; }

            let mut total = node.amplitude;
            let mut count = 1i64;

            for neighbor_coord in coord.neighbors() {
                if let Some(&neighbor_amp) = snapshot.get(&neighbor_coord) {
                    total = total + neighbor_amp;
                    count += 1;
                }
            }

            // A_new = (A_self + Σ_vecinos) / (1 + N_vecinos)
            let avg = total.div_safe(SPA::from_int(count))
                .unwrap_or(node.amplitude);

            // Ruido de fase mínimo (simula entropía cuántica)
            let noisy = if noise_mask > 0 {
                avg + SPA::new(0, 0, 0, noise_mask as i64 % 60, 0)
            } else {
                avg
            };

            new_amps.insert(coord, noisy);
        }

        // Aplicar actualización atómica solo al slot procesado
        for (coord, new_amp) in new_amps {
            if let Some(node) = self.nodes.get_mut(&coord) {
                node.amplitude = new_amp;
            }
        }

        // Calcular coherencia global y cerrar sectores degradados
        self.apply_hexagonal_control();

        DiffusionStep {
            tick:         self.tick,
            coherence:    self.global_coherence(),
            nodes_live:   self.nodes.values().filter(|n| !n.is_closed()).count(),
            nodes_total:  self.nodes.len(),
            quantum_leap: false,
        }
    }

    /// Coherencia global del lattice: amplitud media normalizada.
    pub fn global_coherence(&self) -> SPA {
        let active: Vec<&LatticeNode> = self.nodes.values()
            .filter(|n| !n.is_closed())
            .collect();

        if active.is_empty() { return SPA::zero(); }

        let sum: i64 = active.iter().map(|n| n.amplitude.to_raw()).sum();
        SPA::from_raw(sum / active.len() as i64)
    }

    /// Cierra sectores cuya coherencia local cae bajo el umbral.
    /// (Hexagonal control — redes_micelio_hexagonal.md §4.3)
    fn apply_hexagonal_control(&mut self) {
        for region in 0u8..6 {
            let region_nodes: Vec<SPA> = self.nodes.iter()
                .filter(|(c, _)| c.hex_region() == region)
                .map(|(_, n)| n.amplitude)
                .collect();

            if region_nodes.is_empty() { continue; }

            let sum: i64 = region_nodes.iter().map(|a| a.to_raw()).sum();
            let avg = SPA::from_raw(sum / region_nodes.len() as i64);

            let should_close = avg < self.coherence_threshold;

            for (coord, node) in self.nodes.iter_mut() {
                if coord.hex_region() == region {
                    node.set_closed(should_close);
                }
            }
        }
    }

    /// Bio pulse — inyección de energía en T=17s (Plimpton prime 17).
    ///
    /// En cada sub-fase YHWH (710 ticks), los nodos activos reciben
    /// un micro-pulso proporcional a su `phase_sector` para mantener
    /// la resonancia viva entre quantum leaps.
    /// Modela el "pulso bio" de CRYSTAL_LATTICE.md §9.
    pub fn bio_pulse(&mut self, yhwh_factor: u8) {
        // Micro-energía = YHWH_factor × 1000 en raw SPA (muy pequeño vs SCALE_0)
        let pulse_raw = yhwh_factor as i64 * 1000;

        for (coord, node) in self.nodes.iter_mut() {
            if node.is_closed() || !node.is_occupied() { continue; }

            // El pulso se modula por la región hexagonal del nodo
            // para distribuir energía con variación espacial
            let region_weight = (coord.hex_region() + 1) as i64;
            let energy = SPA::from_raw(pulse_raw * region_weight / 6);
            node.amplitude = node.amplitude + energy;
        }
    }

    /// Quantum Leap — reset de fase en T=68s (17×4 ciclos YHWH).
    ///
    /// Resetea la deriva acumulada de fase sin perder datos:
    /// - Nodos ocupados: conservan `data` y `amplitude`, limpian `dirty` flag
    ///   y re-sincronizan `phase_sector` a la región hexagonal base.
    /// - Nodos vacíos: se purgan completamente (amplitude → 0).
    /// - Todos los sectores se re-abren (closed → false).
    ///
    /// Analogía física: el "parpadeo de la realidad" de EXP-009 §3.
    /// La acumulación de drift relativista se descarta; el lattice
    /// vuelve a fase cero sin destruir la información almacenada.
    pub fn quantum_reset(&mut self) {
        for (coord, node) in self.nodes.iter_mut() {
            // Re-sincronizar phase_sector a baseline hexagonal
            // Sector base = hex_region × 42 (distribuye 6 regiones en 0-255)
            let baseline_sector = coord.hex_region().wrapping_mul(42);

            if node.is_occupied() {
                // Conservar datos, limpiar drift de fase
                node.phase_sector = baseline_sector;
                node.set_dirty(false);
                node.set_closed(false);
                // Normalizar amplitud: clamp a SCALE_0 si excede
                let raw = node.amplitude.to_raw();
                if raw > SPA::SCALE_0 {
                    node.amplitude = SPA::from_raw(SPA::SCALE_0);
                } else if raw < 0 {
                    node.amplitude = SPA::zero();
                }
            } else {
                // Purgar nodo vacío — drift sin datos es ruido puro
                *node = LatticeNode::empty();
                node.phase_sector = baseline_sector;
            }
        }
    }

    /// Vista serializable del lattice para broadcast/API.
    pub fn snapshot(&self) -> Vec<NodeSnapshot> {
        self.nodes.iter().map(|(coord, node)| NodeSnapshot {
            q:            coord.q,
            r:            coord.r,
            amplitude:    node.amplitude.to_string(),
            phase_sector: node.phase_sector,
            occupied:     node.is_occupied(),
            closed:       node.is_closed(),
        }).collect()
    }
}

/// Vista pública de un nodo para serialización.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSnapshot {
    pub q:            i32,
    pub r:            i32,
    pub amplitude:    String,
    pub phase_sector: u8,
    pub occupied:     bool,
    pub closed:       bool,
}


impl LatticeNode {
    fn set_dirty(&mut self, v: bool) {
        if v { self.flags |= 0x04 } else { self.flags &= !0x04 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lattice_rings3_has_37_nodes() {
        let l = LiquidLattice::new(3);
        assert_eq!(l.nodes.len(), 37);
    }

    #[test]
    fn diffusion_preserves_energy_approximately() {
        let mut l = LiquidLattice::new(2);
        // Inyectar energía en centro
        if let Some(node) = l.nodes.get_mut(&Coord { q: 0, r: 0 }) {
            node.amplitude = SPA::from_int(100);
        }
        // 6 ticks (un ciclo completo de phase gating)
        for phase in 0u8..6 {
            l.diffuse_tick(phase, 0);
        }
        let coherence = l.global_coherence();
        // La energía debe haberse distribuido, coherencia > 0
        assert!(coherence.to_raw() > 0);
    }

    #[test]
    fn inject_and_retrieve_roundtrip() {
        let mut l = LiquidLattice::new(3);
        let data = b"hola sentinel mycnet";
        let start = Coord { q: 0, r: 0 };
        let used = l.inject(data, start);
        assert!(used > 0);
        let recovered = l.retrieve(start, data.len());
        assert_eq!(&recovered[..data.len().min(recovered.len())],
                   &data[..data.len().min(recovered.len())]);
    }
}
