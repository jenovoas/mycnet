// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::lattice::{Coord, DiffusionStep, LiquidLattice};
use crate::phase_gate::PhaseGate;
use crate::sync::SatelliteSync;
use mycnet_core::s60::{HolographicSeedBinary, SPA};
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

/// Memoria KV exacta y volátil, separada de la proyección legacy.
pub const MAX_KV_KEYS: usize = 4096;
pub const MAX_KV_KEY_BYTES: usize = 512;

pub fn kv_key_allowed(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_KV_KEY_BYTES
        && (key.starts_with("crystal-") || key.starts_with("portal-metrics-"))
}

/// El orden de campos define LWW total: timestamp, hash y semilla binaria.
/// Solo metadatos: no contiene ni permite recuperar el payload original.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KvSeedRecord {
    pub timestamp_ns: u64,
    pub payload_hash: [u8; 32],
    pub seed_bytes: [u8; 32],
}

impl KvSeedRecord {
    pub fn seed(&self) -> Result<HolographicSeedBinary, &'static str> {
        // Validar en i64 antes del parser legacy, cuya suma cúbica usa i32.
        let q = i32::from_le_bytes(self.seed_bytes[4..8].try_into().unwrap());
        let r = i32::from_le_bytes(self.seed_bytes[8..12].try_into().unwrap());
        let s = i32::from_le_bytes(self.seed_bytes[12..16].try_into().unwrap());
        if i64::from(q) + i64::from(r) + i64::from(s) != 0 {
            return Err("Coordenadas KV inválidas");
        }
        // La suma intermedia del parser también debe ser representable.
        if q.checked_add(r).is_none() {
            return Err("Coordenadas KV fuera de rango");
        }
        HolographicSeedBinary::from_bytes(&self.seed_bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvApply {
    Applied,
    Duplicate,
    Superseded,
}

/// Resultado de una operación de almacenamiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResult {
    pub ok: bool,
    pub nodes_used: usize,
    pub coherence: String,
    pub tick: u64,
}

/// Estado del store para monitoreo externo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStatus {
    pub tick: u64,
    pub coherence: String,
    pub nodes_live: usize,
    pub nodes_total: usize,
    pub phase: String,
    pub yhwh_phase: String,
    pub satellite_id: u8,
    /// Número de Quantum Leaps (T=68s) completados desde el inicio
    pub leap_count: u64,
    /// Ticks hasta el próximo Quantum Leap
    pub ticks_to_leap: u64,
}

/// Motor de almacenamiento resonante de un satélite.
///
/// Cada satélite tiene su instancia local. Los datos se distribuyen
/// automáticamente al diffundir hacia los vecinos vía `SatelliteSync`.
pub struct ResonantStore {
    pub lattice: LiquidLattice,
    pub gate: PhaseGate,
    pub sync: SatelliteSync,
    /// Índice de claves → coordenada de inicio en el lattice
    key_index: HashMap<Vec<u8>, Coord>,
    /// Longitudes originales de datos (para retrieve exacto)
    key_lengths: HashMap<Vec<u8>, usize>,
    /// Marca temporal LWW por clave (mayor tick/timestamp gana).
    key_timestamps: HashMap<Vec<u8>, u64>,
    /// Última versión exacta KV por clave; no comparte índices con `put`.
    kv_seeds: HashMap<String, KvSeedRecord>,
    /// Contador de Quantum Leaps ocurridos (telemetría)
    pub leap_count: u64,
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
            lattice: LiquidLattice::new(rings),
            gate: PhaseGate::new(satellite_id),
            sync: SatelliteSync::new(satellite_id),
            key_index: HashMap::new(),
            key_lengths: HashMap::new(),
            key_timestamps: HashMap::new(),
            kv_seeds: HashMap::new(),
            leap_count: 0,
        }
    }

    /// Almacena datos bajo una clave en el lattice.
    ///
    /// La clave se hashea a una coordenada de inicio.
    /// Los datos se dispersan en espiral desde ese punto.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> StoreResult {
        let start = self.key_to_coord(key);
        let used = self.lattice.inject(value, start);

        self.key_index.insert(key.to_vec(), start);
        self.key_lengths.insert(key.to_vec(), value.len());

        StoreResult {
            ok: used > 0,
            nodes_used: used,
            coherence: self.lattice.global_coherence().to_string(),
            tick: self.lattice.tick,
        }
    }

    /// Almacena con marca temporal para resolución last-writer-wins.
    /// Solo sobrescribe si `timestamp` es mayor al registrado; si no hay
    /// marca previa, equivale a `put`. Retorna true si escribió.
    pub fn put_timestamped(&mut self, key: &[u8], value: &[u8], timestamp: u64) -> bool {
        if let Some(&prev) = self.key_timestamps.get(key) {
            if timestamp <= prev {
                return false;
            }
        }
        self.put(key, value);
        self.key_timestamps.insert(key.to_vec(), timestamp);
        true
    }

    /// Aplica LWW KV antes de confirmar al peer. Duplicados y versiones
    /// superadas no vuelven a inyectarse, aunque la proyección haya disipado.
    /// Si no se puede aplicar, no registra versión ni autoriza un ACK.
    pub fn put_kv_seed(
        &mut self,
        key: &str,
        record: KvSeedRecord,
    ) -> Result<KvApply, &'static str> {
        if !kv_key_allowed(key) {
            return Err("Clave KV fuera de whitelist");
        }
        let seed = record.seed()?;
        if let Some(previous) = self.kv_seeds.get_mut(key) {
            match record.cmp(previous) {
                std::cmp::Ordering::Less => return Ok(KvApply::Superseded),
                std::cmp::Ordering::Equal => return Ok(KvApply::Duplicate),
                std::cmp::Ordering::Greater => {
                    self.lattice.inject_fractal_seed(&seed)?;
                    *previous = record;
                    return Ok(KvApply::Applied);
                }
            }
        }
        if self.kv_seeds.len() >= MAX_KV_KEYS {
            return Err("Capacidad de claves KV agotada");
        }
        self.lattice.inject_fractal_seed(&seed)?;
        self.kv_seeds.insert(key.to_owned(), record);
        Ok(KvApply::Applied)
    }

    /// Semilla original byte a byte, no una reconstrucción desde el lattice.
    pub fn get_kv_seed(&self, key: &str) -> Option<&KvSeedRecord> {
        self.kv_seeds.get(key)
    }

    /// Recupera datos por clave.
    /// Retorna None si la clave no existe o la coherencia es insuficiente.
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let start = *self.key_index.get(key)?;
        let length = *self.key_lengths.get(key)?;
        let data = self.lattice.retrieve(start, length);
        if data.is_empty() {
            None
        } else {
            Some(data)
        }
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
        let is_quantum_leap = crystal_tick > 0 && crystal_tick.is_multiple_of(QUANTUM_LEAP_TICKS);
        if is_quantum_leap {
            self.lattice.quantum_reset();
            self.leap_count += 1;
        }

        let yhwh = YhwhPhase::current();
        let gate_state = self.gate.advance(crystal_tick, yhwh.factor());

        // Bio pulse: cada 710 ticks (T=17s, primo Plimpton)
        // Inyecta micro-energía para mantener resonancia entre quantum leaps
        // Salta si es un Quantum Leap (el reset ya restaura el estado)
        if !is_quantum_leap && crystal_tick > 0 && crystal_tick.is_multiple_of(BIO_PULSE_TICKS) {
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
        let mut step = self
            .lattice
            .diffuse_tick(gate_state.active_region.as_u8(), noise);
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
            coherence: step_info.to_string(),
            nodes_live: self
                .lattice
                .nodes
                .values()
                .filter(|n| !n.is_closed())
                .count(),
            nodes_total: self.lattice.nodes.len(),
            phase: format!("{}", self.gate.crystal_tick % 6),
            yhwh_phase: yhwh.name().to_string(),
            satellite_id: self.gate.satellite_id,
            leap_count: self.leap_count,
            ticks_to_leap,
        }
    }

    /// Convierte una clave a coordenada axial del lattice.
    /// Hash simple: XOR de bytes → (q, r) dentro del lattice.
    fn key_to_coord(&self, key: &[u8]) -> Coord {
        let h: u32 = key
            .iter()
            .enumerate()
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
        store.put(b"key_beta", b"valor_dos");
        assert!(store.get(b"key_alpha").is_some());
        assert!(store.get(b"key_beta").is_some());
    }
    #[test]
    fn put_timestamped_last_writer_wins() {
        let mut store = ResonantStore::new(0, 6);
        assert!(store.put_timestamped(b"k", b"viejo", 10));
        assert!(!store.put_timestamped(b"k", b"atrasado", 5));
        assert!(!store.put_timestamped(b"k", b"igual", 10));
        assert!(store.put_timestamped(b"k", b"nuevo", 20));
        let data = store.get(b"k").expect("la clave debe existir");
        assert_eq!(&data[..5], b"nuevo");
    }
}

#[cfg(test)]
mod kv_tests {
    use super::*;

    fn record(timestamp_ns: u64, hash: u8, token: u32) -> KvSeedRecord {
        let mut seed = HolographicSeedBinary::from_lfm(token, 0, 0, 17);
        seed.reserved = [0xab, 0xcd];
        KvSeedRecord {
            timestamp_ns,
            payload_hash: [hash; 32],
            seed_bytes: seed.to_bytes(),
        }
    }

    #[test]
    fn kv_retains_exact_seed_separately_from_legacy_and_dissipation() {
        let mut store = ResonantStore::new(0, 3);
        let value = record(123, 7, 456);
        assert_eq!(
            store.put_kv_seed("crystal-exact", value),
            Ok(KvApply::Applied)
        );
        store.put(b"crystal-exact", b"legacy overwrite");
        store.put_timestamped(b"crystal-exact", b"legacy timestamp", u64::MAX);
        store.tick(1);
        assert_eq!(store.get_kv_seed("crystal-exact"), Some(&value));
        assert_eq!(
            store.get_kv_seed("crystal-exact").unwrap().seed_bytes[30..],
            [0xab, 0xcd]
        );
    }

    #[test]
    fn kv_equal_timestamps_converge_by_hash_then_seed_in_any_order() {
        let versions = [
            record(9, 255, 9),
            record(10, 1, 1),
            record(10, 2, 1),
            record(10, 2, 2),
        ];
        let expected = *versions.iter().max().unwrap();
        for order in [[0, 1, 2, 3], [3, 2, 1, 0], [2, 0, 3, 1], [1, 3, 0, 2]] {
            let mut store = ResonantStore::new(0, 3);
            for index in order {
                store
                    .put_kv_seed("portal-metrics-order", versions[index])
                    .unwrap();
            }
            assert_eq!(store.get_kv_seed("portal-metrics-order"), Some(&expected));
        }
    }

    #[test]
    fn kv_duplicate_and_superseded_do_not_reinject_lattice() {
        let mut store = ResonantStore::new(0, 3);
        let value = record(10, 1, 2);
        store.put_kv_seed("crystal-once", value).unwrap();
        let coord = Coord { q: 0, r: 0 };
        store.lattice.nodes.get_mut(&coord).unwrap().amplitude = SPA::from_raw(123);
        assert_eq!(
            store.put_kv_seed("crystal-once", value),
            Ok(KvApply::Duplicate)
        );
        assert_eq!(
            store.put_kv_seed("crystal-once", record(9, 255, 9)),
            Ok(KvApply::Superseded),
        );
        assert_eq!(store.lattice.nodes[&coord].amplitude, SPA::from_raw(123));
        assert_eq!(store.get_kv_seed("crystal-once"), Some(&value));
    }

    #[test]
    fn kv_capacity_preserves_existing_versions_without_eviction() {
        let mut store = ResonantStore::new(0, 3);
        let original = record(1, 1, 1);
        for i in 0..MAX_KV_KEYS {
            store
                .put_kv_seed(&format!("crystal-{i}"), original)
                .unwrap();
        }
        assert!(store.put_kv_seed("crystal-overflow", original).is_err());
        assert!(store.get_kv_seed("crystal-overflow").is_none());
        assert_eq!(
            store.put_kv_seed("crystal-0", original),
            Ok(KvApply::Duplicate)
        );
        let newer = record(2, 1, 1);
        assert_eq!(store.put_kv_seed("crystal-0", newer), Ok(KvApply::Applied));
        assert_eq!(store.kv_seeds.len(), MAX_KV_KEYS);
        assert_eq!(store.get_kv_seed("crystal-0"), Some(&newer));
    }

    #[test]
    fn kv_rejects_invalid_key_seed_and_unapplicable_projection_atomically() {
        let mut store = ResonantStore::new(0, 3);
        let value = record(1, 1, 1);
        for key in ["security-secret", "genesis", "portal", ""] {
            assert!(store.put_kv_seed(key, value).is_err());
        }
        let mut invalid = value;
        invalid.seed_bytes[0] ^= 1;
        assert!(store.put_kv_seed("crystal-crc", invalid).is_err());
        let outside = KvSeedRecord {
            seed_bytes: HolographicSeedBinary::from_lfm(1, 9, 0, 1).to_bytes(),
            ..value
        };
        assert!(store.put_kv_seed("crystal-outside", outside).is_err());
        assert!(store.kv_seeds.is_empty());
    }
}
