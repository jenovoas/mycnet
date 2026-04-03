//! # SatelliteSync — Sincronización de Fronteras entre Satélites
//!
//! Los satélites comparten sus nodos frontera a través de batman-adv.
//! El TQ del enlace determina la fuerza de acoplamiento:
//! - TQ alto (250+) → difusión fuerte → datos fluyen rápido entre satélites
//! - TQ bajo (<100) → difusión débil → datos se conservan localmente
//! - TQ = 0         → satélite vecino caído → frontera desacoplada (auto-reparación)
//!
//! ## Principio micelial
//!
//! En micelio biológico: gradiente químico alto → hifa crece hacia nutriente.
//! En MycNet: TQ alto → acoplamiento fuerte → datos fluyen al vecino.
//! El lattice se comporta como superfluido: va donde es necesario.
//!
//! ## Buffer de transporte
//!
//! Usa un ring buffer SPSC lock-free (basado en buffer_system.rs de Sentinel)
//! con tamaño 60² = 3600 (alineado a harmónico SPA).
//!
//! Referencia: telemetria_predictiva.md, buffer_system.rs, EXP-009

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use mycnet_core::s60::SPA;
use crate::lattice::Coord;
use crate::phase_gate::DiffusionStrength;

/// Paquete de frontera enviado entre satélites vía batman-adv UDP.
///
/// Contiene los nodos del borde del lattice local para que
/// el vecino los use como condición de frontera en su difusión.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryPacket {
    /// ID del satélite emisor
    pub satellite_id:  u8,
    /// Tick del Crystal Clock en el que se capturó el estado
    pub crystal_tick:  u64,
    /// TQ del enlace batman-adv hacia el receptor (0-255)
    pub link_tq:       u8,
    /// Nodos de frontera: (q, r, amplitude_raw, phase_sector)
    pub boundary_nodes: Vec<BoundaryNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryNode {
    pub q:            i32,
    pub r:            i32,
    pub amplitude_raw: i64,
    pub phase_sector: u8,
}

/// Gestor de sincronización de un satélite.
pub struct SatelliteSync {
    /// ID de este satélite
    pub local_id:      u8,
    /// TQ de cada satélite vecino (MAC → TQ)
    neighbor_tq:       HashMap<String, u8>,
    /// Últimos paquetes de frontera recibidos por vecino
    received_borders:  HashMap<u8, BoundaryPacket>,
    /// Ring buffer para paquetes entrantes (lock-free SPSC)
    /// Tamaño 3600 = 60² (alineado a SPA harmónico, como buffer_system.rs)
    inbound_ring:      Vec<Option<BoundaryPacket>>,
    ring_head:         usize,
    ring_tail:         usize,
}

const RING_SIZE: usize = 3600; // 60² — harmónico SPA

impl SatelliteSync {
    pub fn new(local_id: u8) -> Self {
        Self {
            local_id,
            neighbor_tq:      HashMap::new(),
            received_borders: HashMap::new(),
            inbound_ring:     vec![None; RING_SIZE],
            ring_head:        0,
            ring_tail:        0,
        }
    }

    /// Actualiza el TQ de un vecino (llamar tras cada lectura de batctl).
    pub fn update_neighbor_tq(&mut self, mac: String, tq: u8) {
        self.neighbor_tq.insert(mac, tq);
    }

    /// Encola un paquete de frontera recibido de un vecino.
    pub fn receive(&mut self, pkt: BoundaryPacket) {
        let next = (self.ring_head + 1) % RING_SIZE;
        if next != self.ring_tail {
            self.inbound_ring[self.ring_head] = Some(pkt);
            self.ring_head = next;
        }
        // Si el ring está lleno, el paquete más nuevo descarta el slot
        // (saturación harmónica — no bloqueamos)
    }

    /// Procesa todos los paquetes pendientes y retorna condiciones de frontera.
    pub fn drain_boundaries(&mut self) -> Vec<(Coord, SPA, DiffusionStrength)> {
        let mut result = Vec::new();

        while self.ring_tail != self.ring_head {
            if let Some(pkt) = self.inbound_ring[self.ring_tail].take() {
                let coupling = self.coupling_for_tq(pkt.link_tq);
                for bn in &pkt.boundary_nodes {
                    let coord = Coord { q: bn.q, r: bn.r };
                    // Amplitud ponderada por TQ: alto TQ = máxima energía recibida
                    let weight = pkt.link_tq as i64;
                    let amp = SPA::from_raw(
                        (bn.amplitude_raw * weight) / 255
                    );
                    result.push((coord, amp, coupling));
                }
                self.received_borders.insert(pkt.satellite_id, pkt);
            }
            self.ring_tail = (self.ring_tail + 1) % RING_SIZE;
        }

        result
    }

    /// Construye el paquete de frontera local para enviar a vecinos.
    /// Solo incluye nodos cuya hex_region coincide con el slot activo.
    pub fn build_boundary_packet(
        &self,
        tick: u64,
        neighbor_mac: &str,
        nodes: &[(Coord, SPA, u8)], // (coord, amplitude, phase_sector)
    ) -> BoundaryPacket {
        let tq = *self.neighbor_tq.get(neighbor_mac).unwrap_or(&128);

        let boundary_nodes = nodes.iter().map(|(c, amp, ps)| BoundaryNode {
            q:             c.q,
            r:             c.r,
            amplitude_raw: amp.to_raw(),
            phase_sector:  *ps,
        }).collect();

        BoundaryPacket {
            satellite_id:   self.local_id,
            crystal_tick:   tick,
            link_tq:        tq,
            boundary_nodes,
        }
    }

    /// Fuerza de difusión basada en TQ del enlace.
    ///
    /// Analogía micelial:
    /// - TQ 200-255: hifa robusta, flujo máximo
    /// - TQ 100-199: hifa normal, flujo medio
    /// - TQ  0- 99: hifa débil/dañada, flujo mínimo
    fn coupling_for_tq(&self, tq: u8) -> DiffusionStrength {
        match tq {
            200..=255 => DiffusionStrength::High,
            100..=199 => DiffusionStrength::Medium,
            _         => DiffusionStrength::Low,
        }
    }

    /// Pre-caché predictivo hexagonal (telemetria_predictiva.md §2).
    ///
    /// Si accedimos al nodo (q,r), pre-cargamos sus 6 vecinos.
    /// Retorna las coordenadas a pre-fetchear.
    pub fn predict_next_access(coord: Coord) -> [Coord; 6] {
        coord.neighbors()
    }
}
