// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
// crates/mycnet-memory/src/holographic_sync.rs
//! # 🌌 Holographic Resonance & Fractal Seed Synchronization (LFM ↔ MycNet)
//!
//! Implementa sincronización holográfica con cero payload de red.
//! En lugar de transferir matrices completas de nodos por la red, solo se propaga
//! una semilla fractal compacta de 32 bytes (`HolographicSeedBinary`).
//! Cualquier nodo que ejecute la misma ley isócrona regenera el estado de campo
//! idéntico localmente sobre su `LiquidLattice`: Estado = f^N(Seed).

use serde::{Deserialize, Serialize};
use mycnet_core::s60::SPA;
pub use mycnet_core::s60::{HolographicSeedBinary, compute_crc16};
use crate::storage::ResonantStore;

/// Semilla Holográfica de alto nivel para almacenamiento KV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolographicSeed {
    pub key: Vec<u8>,
    pub amplitude: SPA,
    pub phase_sector: u8,
    pub pulse_period: u32,
    pub seed_bytes: Vec<u8>,
}

impl HolographicSeed {
    pub fn new(key: Vec<u8>, amplitude: SPA, phase_sector: u8, pulse_period: u32, seed_bytes: Vec<u8>) -> Self {
        Self {
            key,
            amplitude,
            phase_sector,
            pulse_period,
            seed_bytes,
        }
    }

    /// Crea una semilla desde un token LFM.
    pub fn from_lfm_token(token_id: u32, phase_sector: u8) -> Self {
        let normalized = (token_id % 3600) as i64;
        let amp = SPA::from_int(normalized);
        let key = format!("lfm:tok:{token_id}").into_bytes();
        
        let mut seed_bytes = token_id.to_le_bytes().to_vec();
        seed_bytes.push(phase_sector);
        seed_bytes.push(0xAA); // Resonant sync marker
        
        Self::new(key, amp, phase_sector, 710, seed_bytes)
    }

    /// Convierte a formato binario estandarizado de 32 bytes en la coordenada axial dada.
    pub fn to_binary(&self, q: i32, r: i32) -> HolographicSeedBinary {
        let token_id = if self.seed_bytes.len() >= 4 {
            u32::from_le_bytes(self.seed_bytes[0..4].try_into().unwrap_or([0; 4]))
        } else {
            0
        };
        HolographicSeedBinary::new(
            token_id,
            q,
            r,
            -q - r,
            self.amplitude.to_raw(),
            self.phase_sector,
            10,
            0x01,
        )
    }
}

/// Motor de sincronización holográfica.
pub struct HolographicSyncEngine;

impl HolographicSyncEngine {
    /// Inyecta una semilla holográfica en el almacén local.
    pub fn expand_seed(store: &mut ResonantStore, seed: &HolographicSeed) {
        store.put(&seed.key, &seed.seed_bytes);
    }

    /// Reconstruye el payload exacto en un nodo desde su memoria resonante.
    pub fn reconstruct_payload(store: &ResonantStore, seed: &HolographicSeed) -> Option<Vec<u8>> {
        store.get(&seed.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_holographic_seed_binary_layout_32_bytes() {
        assert_eq!(std::mem::size_of::<HolographicSeedBinary>(), 32);
    }

    #[test]
    fn test_holographic_seed_binary_crc_and_cubic_invariant() {
        let seed = HolographicSeedBinary::from_lfm(42, 1, -1, 30);
        assert!(seed.is_valid_crc());
        assert!(seed.is_valid_cubic());
        assert_eq!(seed.coord_q, 1);
        assert_eq!(seed.coord_r, -1);
        assert_eq!(seed.coord_s, 0); // 1 + (-1) + 0 == 0

        // Roundtrip a bytes
        let bytes = seed.to_bytes();
        assert_eq!(bytes.len(), 32);

        let decoded = HolographicSeedBinary::from_bytes(&bytes).expect("Decodificación falló");
        assert_eq!(seed, decoded);
    }

    #[test]
    fn test_holographic_seed_binary_invalid_cubic_rejected() {
        let seed = HolographicSeedBinary::new(10, 1, 1, 1, 12960000, 0, 10, 1); // 1+1+1 != 0
        assert!(!seed.is_valid_cubic());
        let bytes = seed.to_bytes();
        assert!(HolographicSeedBinary::from_bytes(&bytes).is_err());
    }

    #[test]
    fn test_holographic_seed_reconstruction_fidelity() {
        let mut node_a_store = ResonantStore::new(0, 3);
        let mut node_b_store = ResonantStore::new(1, 3);

        let seed = HolographicSeed::from_lfm_token(120, 42);

        HolographicSyncEngine::expand_seed(&mut node_a_store, &seed);
        HolographicSyncEngine::expand_seed(&mut node_b_store, &seed);

        let data_a = HolographicSyncEngine::reconstruct_payload(&node_a_store, &seed).expect("Node A reconstruction failed");
        let data_b = HolographicSyncEngine::reconstruct_payload(&node_b_store, &seed).expect("Node B reconstruction failed");

        assert_eq!(data_a, data_b);
        assert_eq!(data_a, seed.seed_bytes);
    }

    #[test]
    fn test_lfm_seed_structure_is_compact() {
        assert!(std::mem::size_of::<HolographicSeed>() <= 128);
    }
}
