//! # ResonantStore — API pública de almacenamiento
//!
//! Interfaz de alto nivel para guardar/recuperar datos en el lattice.
//! Los datos fluyen como superfluido a través de la malla de satélites.
//!
//! ## Modelo de consistencia micelial
//!
//! No es consistencia fuerte ni eventual estándar.
//! Es **consistencia por coherencia**: un dato es "consistente"
//! cuando su amplitud en el lattice supera el umbral S60 de coherencia.
//!
//! La lectura siempre retorna el dato de mayor amplitud disponible
//! en los nodos del cluster visible desde este satélite.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use mycnet_core::s60::SPA;
use crate::lattice::{LiquidLattice, Coord, DiffusionStep};
use crate::phase_gate::PhaseGate;
use crate::sync::SatelliteSync;
use mycnet_core::yhwh::YhwhPhase;

/// Ticks por Quantum Leap: T=68s × 41.77 Hz ≈ 2840 ticks.
/// Derivado de 17 (Plimpton prime) × 4 (fases YHWH) = 68.
/// El lattice resetea fase acumulada sin destruir datos.
pub const QUANTUM_LEAP_TICKS: u64 = 2840;

/// Bio pulse: T=17s × 41.77 Hz ≈ 710 ticks (QUANTUM_LEAP_TICKS / 4).
/// En cada sub-fase YHWH, el lattice recibe un micro-pulso de energía
/// para mantener la resonancia viva entre quantum leaps.
/// Deriva del número primo 17 de la Tabla Plimpton 322.
pub const BIO_PULSE_TICKS: u64 = 710;

/// Resultado de una operación de almacenamiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResult {
    pub ok:          bool,
    pub nodes_used:  usize,
    pub coherence:   String,
    pub tick:        u64,
}

/// Estado del store para monitoreo externo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStatus {
    pub tick:            u64,
    pub coherence:       String,
    pub nodes_live:      usize,
    pub nodes_total:     usize,
    pub phase:           String,
    pub yhwh_phase:      String,
    pub satellite_id:    u8,
    /// Número de Quantum Leaps (T=68s) completados desde el inicio
    pub leap_count:      u64,
    /// Ticks hasta el próximo Quantum Leap
    pub ticks_to_leap:   u64,
}

/// Motor de almacenamiento resonante de un satélite.
///
/// Cada satélite tiene su instancia local. Los datos se distribuyen
/// automáticamente al diffundir hacia los vecinos vía `SatelliteSync`.
pub struct ResonantStore {
    pub lattice:        LiquidLattice,
    pub gate:           PhaseGate,
    pub sync:           SatelliteSync,
    /// Índice de claves → coordenada de inicio en el lattice
    key_index:          HashMap<Vec<u8>, Coord>,
    /// Longitudes originales de datos (para retrieve exacto)
    key_lengths:        HashMap<Vec<u8>, usize>,
    /// Contador de Quantum Leaps ocurridos (telemetría)
    pub leap_count:     u64,
}

impl ResonantStore {
    /// Crea un nuevo store para el satélite `satellite_id`.
    ///
    /// `rings` determina la capacidad:
    /// - rings=3 → 37 nodos → 222 bytes por satélite
    /// - rings=6 → 127 nodos → 762 bytes por satélite
    /// - rings=12 → 469 nodos → 2.8 KB por satélite
    pub fn new(satellite_id: u8, rings: u32) -> Self {
        Self {
            lattice:     LiquidLattice::new(rings),
            gate:        PhaseGate::new(satellite_id),
            sync:        SatelliteSync::new(satellite_id),
            key_index:   HashMap::new(),
            key_lengths: HashMap::new(),
            leap_count:  0,
        }
    }

    /// Almacena datos bajo una clave en el lattice.
    ///
    /// La clave se hashea a una coordenada de inicio.
    /// Los datos se dispersan en espiral desde ese punto.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> StoreResult {
        let start = self.key_to_coord(key);
        let used  = self.lattice.inject(value, start);

        self.key_index.insert(key.to_vec(), start);
        self.key_lengths.insert(key.to_vec(), value.len());

        StoreResult {
            ok:         used > 0,
            nodes_used: used,
            coherence:  self.lattice.global_coherence().to_string(),
            tick:       self.lattice.tick,
        }
    }

    /// Recupera datos por clave.
    /// Retorna None si la clave no existe o la coherencia es insuficiente.
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let start  = *self.key_index.get(key)?;
        let length = *self.key_lengths.get(key)?;
        let data   = self.lattice.retrieve(start, length);
        if data.is_empty() { None } else { Some(data) }
    }

    /// Tick principal del store.
    ///
    /// Llamar a esta función cada tick del Crystal Clock (41.77 Hz).
    /// 1. Detecta Quantum Leap (T=68s = 2840 ticks, 17×4 YHWH)
    /// 2. Avanza el phase gate según el tick global
    /// 3. Aplica difusión al slot activo del lattice
    /// 4. Integra condiciones de frontera recibidas de vecinos
    /// 5. Retorna el estado del paso de difusión
    pub fn tick(&mut self, crystal_tick: u64) -> DiffusionStep {
        // Quantum Leap: en cada frontera de 2840 ticks (T=68s)
        // el lattice resetea la deriva de fase — "parpadeo" sin pérdida de datos
        let is_quantum_leap = crystal_tick > 0 && crystal_tick % QUANTUM_LEAP_TICKS == 0;
        if is_quantum_leap {
            self.lattice.quantum_reset();
            self.leap_count += 1;
        }

        let yhwh = YhwhPhase::current();
        let gate_state = self.gate.advance(crystal_tick, yhwh.factor());

        // Bio pulse: cada 710 ticks (T=17s, primo Plimpton)
        // Inyecta micro-energía para mantener resonancia entre quantum leaps
        // Salta si es un Quantum Leap (el reset ya restaura el estado)
        if !is_quantum_leap && crystal_tick > 0 && crystal_tick % BIO_PULSE_TICKS == 0 {
            self.lattice.bio_pulse(yhwh.factor());
        }

        // Integrar fronteras recibidas de satélites vecinos
        let boundaries = self.sync.drain_boundaries();
        for (coord, amp, strength) in boundaries {
            if let Some(node) = self.lattice.nodes.get_mut(&coord) {
                // Acoplamiento micelial: A_new = A_local*(1-w) + A_vecino*w
                let w = strength.coupling_weight() as i64;
                let local = node.amplitude.to_raw();
                let remote = amp.to_raw();
                let blended = (local * (255 - w) + remote * w) / 255;
                node.amplitude = SPA::from_raw(blended);
            }
        }

        // Difusión del slot activo (phase gating)
        // Ruido mínimo: bit bajo del tick como seed de entropía
        let noise = (crystal_tick & 0x07) as u8;
        let mut step = self.lattice.diffuse_tick(gate_state.active_region.as_u8(), noise);
        step.quantum_leap = is_quantum_leap;
        step
    }

    /// Estado del store para telemetría y API.
    pub fn status(&self) -> StoreStatus {
        let step_info = self.lattice.global_coherence();
        let yhwh = YhwhPhase::current();

        let tick = self.lattice.tick;
        let ticks_to_leap = QUANTUM_LEAP_TICKS - (tick % QUANTUM_LEAP_TICKS);

        StoreStatus {
            tick,
            coherence:      step_info.to_string(),
            nodes_live:     self.lattice.nodes.values().filter(|n| !n.is_closed()).count(),
            nodes_total:    self.lattice.nodes.len(),
            phase:          format!("{}", self.gate.crystal_tick % 6),
            yhwh_phase:     yhwh.name().to_string(),
            satellite_id:   self.gate.satellite_id,
            leap_count:     self.leap_count,
            ticks_to_leap,
        }
    }

    /// Convierte una clave a coordenada axial del lattice.
    /// Hash simple: XOR de bytes → (q, r) dentro del lattice.
    fn key_to_coord(&self, key: &[u8]) -> Coord {
        let h: u32 = key.iter().enumerate()
            .fold(0u32, |acc, (i, &b)| acc ^ ((b as u32) << (i % 24)));

        let r_max = self.lattice.rings as i32;
        let q = ((h & 0xFF) as i32 % (2 * r_max + 1)) - r_max;
        let r = (((h >> 8) & 0xFF) as i32 % (2 * r_max + 1)) - r_max;
        Coord { q, r }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_and_get_roundtrip() {
        let mut store = ResonantStore::new(0, 3);
        let result = store.put(b"test_key", b"hello mycnet");
        assert!(result.ok);

        let recovered = store.get(b"test_key");
        assert!(recovered.is_some());
        let data = recovered.unwrap();
        assert_eq!(&data[..12], b"hello mycnet");
    }

    #[test]
    fn tick_advances_lattice() {
        let mut store = ResonantStore::new(0, 3);
        store.put(b"k", b"data_for_tick_test");
        let step = store.tick(1);
        assert_eq!(step.tick, 1);
    }

    #[test]
    fn multiple_keys_coexist() {
        let mut store = ResonantStore::new(0, 6);
        store.put(b"key_alpha", b"valor_uno");
        store.put(b"key_beta",  b"valor_dos");
        assert!(store.get(b"key_alpha").is_some());
        assert!(store.get(b"key_beta").is_some());
    }
}
