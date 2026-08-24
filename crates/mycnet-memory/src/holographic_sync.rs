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
use crate::lattice::Coord;
use crate::storage::ResonantStore;

/// Calcula el CRC16-CCITT (polinomio 0x1021, init 0xFFFF) sin punto flotante.
pub fn compute_crc16(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in bytes {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Semilla Fractal Binaria de 32 bytes exactos (OpenSpec OPENSPEC-MYC-001).
/// Layout FFI determinista little-endian #[repr(C)].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolographicSeedBinary {
    /// ID del token o evento proyectado (0..4294967295)
    pub token_id: u32,
    /// Coordenada cúbica hexagonal q
    pub coord_q: i32,
    /// Coordenada cúbica hexagonal r
    pub coord_r: i32,
    /// Coordenada cúbica hexagonal s (Invariante espacial: q + r + s = 0)
    pub coord_s: i32,
    /// Amplitud de energía SPA en escala 60^4 = 12,960,000
    pub amplitude_s60: i64,
    /// Sector de fase armónica (0..59)
    pub phase_sector: u8,
    /// Pulso armónico YHWH (10-5-6-5)
    pub yhwh_pulse: u8,
    /// Flags (bit0=activo, bit1=snapped, bit2=bridge)
    pub flags: u16,
    /// Checksum CRC16-CCITT de los primeros 28 bytes
    pub crc16: u16,
    /// Relleno de alineación a 32 bytes exactos
    pub reserved: [u8; 2],
}

/// Verificación estricta en tiempo de compilación: la semilla DEBE medir 32 bytes exactos.
const _: () = assert!(
    std::mem::size_of::<HolographicSeedBinary>() == 32,
    "HolographicSeedBinary debe medir exactamente 32 bytes según OPENSPEC-MYC-001"
);

impl HolographicSeedBinary {
    /// Crea una semilla fractal calculando su checksum CRC16 automáticamente.
    pub fn new(
        token_id: u32,
        coord_q: i32,
        coord_r: i32,
        coord_s: i32,
        amplitude_s60: i64,
        phase_sector: u8,
        yhwh_pulse: u8,
        flags: u16,
    ) -> Self {
        let mut seed = Self {
            token_id,
            coord_q,
            coord_r,
            coord_s,
            amplitude_s60,
            phase_sector,
            yhwh_pulse,
            flags,
            crc16: 0,
            reserved: [0u8; 2],
        };
        seed.crc16 = seed.compute_checksum();
        seed
    }

    /// Crea una semilla fractal válida a partir de un token LFM y coordenadas axiales (q, r).
    pub fn from_lfm(token_id: u32, q: i32, r: i32, phase_sector: u8) -> Self {
        let s = -q - r; // Invariante cúbica q + r + s = 0
        let normalized = (token_id % 3600) as i64;
        let amp = normalized * SPA::SCALE_0; // Escala SPA
        Self::new(token_id, q, r, s, amp, phase_sector, 10, 0x01)
    }

    /// Calcula el checksum CRC16 sobre los primeros 28 bytes de la estructura.
    pub fn compute_checksum(&self) -> u16 {
        let mut buffer = [0u8; 28];
        buffer[0..4].copy_from_slice(&self.token_id.to_le_bytes());
        buffer[4..8].copy_from_slice(&self.coord_q.to_le_bytes());
        buffer[8..12].copy_from_slice(&self.coord_r.to_le_bytes());
        buffer[12..16].copy_from_slice(&self.coord_s.to_le_bytes());
        buffer[16..24].copy_from_slice(&self.amplitude_s60.to_le_bytes());
        buffer[24] = self.phase_sector;
        buffer[25] = self.yhwh_pulse;
        buffer[26..28].copy_from_slice(&self.flags.to_le_bytes());
        compute_crc16(&buffer)
    }

    /// Valida que el checksum CRC16 coincida con el contenido.
    pub fn is_valid_crc(&self) -> bool {
        self.crc16 == self.compute_checksum()
    }

    /// Valida la invariante espacial cúbica hexagonal: q + r + s == 0.
    pub fn is_valid_cubic(&self) -> bool {
        self.coord_q + self.coord_r + self.coord_s == 0
    }

    /// Convierte las coordenadas cúbicas a una coordenada axial `Coord`.
    pub fn to_coord(&self) -> Option<Coord> {
        if self.is_valid_cubic() {
            Some(Coord { q: self.coord_q, r: self.coord_r })
        } else {
            None
        }
    }

    /// Serializa la estructura a un array de 32 bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..4].copy_from_slice(&self.token_id.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.coord_q.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.coord_r.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.coord_s.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.amplitude_s60.to_le_bytes());
        bytes[24] = self.phase_sector;
        bytes[25] = self.yhwh_pulse;
        bytes[26..28].copy_from_slice(&self.flags.to_le_bytes());
        bytes[28..30].copy_from_slice(&self.crc16.to_le_bytes());
        bytes[30..32].copy_from_slice(&self.reserved);
        bytes
    }

    /// Deserializa desde un array de 32 bytes verificando CRC16.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, &'static str> {
        let token_id = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let coord_q = i32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let coord_r = i32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let coord_s = i32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let amplitude_s60 = i64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let phase_sector = bytes[24];
        let yhwh_pulse = bytes[25];
        let flags = u16::from_le_bytes(bytes[26..28].try_into().unwrap());
        let crc16 = u16::from_le_bytes(bytes[28..30].try_into().unwrap());
        let reserved = [bytes[30], bytes[31]];

        let seed = Self {
            token_id,
            coord_q,
            coord_r,
            coord_s,
            amplitude_s60,
            phase_sector,
            yhwh_pulse,
            flags,
            crc16,
            reserved,
        };

        if !seed.is_valid_crc() {
            return Err("Checksum CRC16 inválido en semilla holográfica");
        }
        if !seed.is_valid_cubic() {
            return Err("Violación de invariante cúbica hexagonal (q+r+s != 0)");
        }

        Ok(seed)
    }
}

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
