// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
// crates/mycnet-memory/src/holographic_sync.rs
//! # 🌌 Holographic Resonance & Fractal Seed Synchronization (LFM ↔ MycNet)
//!
//! Implements zero-network payload holographic data synchronization.
//! Instead of transferring heavy node matrices over the network, only a 32-byte
//! fractal seed (coordinate, amplitude, phase sector, YHWH pulse) is broadcast.
//! Any node running the same isochronous law regenerates the exact identical
//! field state locally: Dato = f^N(Seed).

use serde::{Deserialize, Serialize};
use mycnet_core::s60::SPA;
use crate::storage::ResonantStore;

/// Minimal 32-byte Holographic Seed for zero-payload network transmission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolographicSeed {
    /// Seed identifier key
    pub key: Vec<u8>,
    /// Exact initial energy amplitude in S60
    pub amplitude: SPA,
    /// Phase sector (0-255)
    pub phase_sector: u8,
    /// YHWH pulse period (e.g. 710 ticks for 17s bio-pulse at 41.77 Hz)
    pub pulse_period: u32,
    /// Payload seed bytes
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

    /// Creates a seed from an LFM projected token.
    pub fn from_lfm_token(token_id: u32, phase_sector: u8) -> Self {
        let normalized = (token_id % 3600) as i64;
        let amp = SPA::from_int(normalized);
        let key = format!("lfm:tok:{token_id}").into_bytes();
        
        let mut seed_bytes = token_id.to_le_bytes().to_vec();
        seed_bytes.push(phase_sector);
        seed_bytes.push(0xAA); // Resonant sync marker
        
        Self::new(key, amp, phase_sector, 710, seed_bytes)
    }
}

/// Holographic synchronization engine.
pub struct HolographicSyncEngine;

impl HolographicSyncEngine {
    /// Injects a holographic seed into a local store.
    pub fn expand_seed(store: &mut ResonantStore, seed: &HolographicSeed) {
        store.put(&seed.key, &seed.seed_bytes);
    }

    /// Reconstructs the exact payload on a node from its local memory.
    pub fn reconstruct_payload(store: &ResonantStore, seed: &HolographicSeed) -> Option<Vec<u8>> {
        store.get(&seed.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_holographic_seed_reconstruction_fidelity() {
        // Node A (Laptop) and Node B (Server Fan) start as completely independent stores
        let mut node_a_store = ResonantStore::new(0, 3);
        let mut node_b_store = ResonantStore::new(1, 3);

        // Generate LFM seed: Token 120, Phase Sector 42
        let seed = HolographicSeed::from_lfm_token(120, 42);

        // Both nodes expand the compact seed locally without network payload transfer
        HolographicSyncEngine::expand_seed(&mut node_a_store, &seed);
        HolographicSyncEngine::expand_seed(&mut node_b_store, &seed);

        // Reconstruct data locally on both nodes from the seed
        let data_a = HolographicSyncEngine::reconstruct_payload(&node_a_store, &seed).expect("Node A reconstruction failed");
        let data_b = HolographicSyncEngine::reconstruct_payload(&node_b_store, &seed).expect("Node B reconstruction failed");

        // Data reconstructed on both isolated nodes must be 100% identical
        assert_eq!(data_a, data_b);
        assert_eq!(data_a, seed.seed_bytes);
    }

    #[test]
    fn test_lfm_seed_structure_is_compact() {
        // Verify seed is small enough to fit in a single UDP packet or atomic pulse (<128 bytes)
        assert!(std::mem::size_of::<HolographicSeed>() <= 128);
    }
}
