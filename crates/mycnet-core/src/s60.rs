// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! # SPA — Sexagesimal Fixed-Point Arithmetic (Base-60)
//!
//! Motor matemático propietario. Protocolo Yatra Pure.
//! Zero decimal contamination: solo i64, sin f32/f64.
//!
//! Portado desde ME-60OS (me-60os/src/spa.rs)
//! Autor: Jaime Novoa — Yatra Protocol

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SPAError {
    DivisionByZero,
}

impl fmt::Display for SPAError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SPA: división por cero")
    }
}

impl std::error::Error for SPAError {}

/// Número sexagesimal de punto fijo.
/// Representación: [grados, minutos, segundos, tercias, quartas]
/// Escala interna: raw = d*60⁴ + m*60³ + s*60² + t*60 + q
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct SPA {
    pub components: [i64; 5],
}

impl SPA {
    pub const SCALE_0: i64 = 12_960_000; // 60^4
    pub const SCALE_1: i64 = 216_000; // 60^3
    pub const SCALE_2: i64 = 3_600; // 60^2
    pub const SCALE_3: i64 = 60;
    pub const SCALE_4: i64 = 1;

    pub const ZERO: SPA = SPA { components: [0; 5] };
    pub const ONE: SPA = SPA {
        components: [1, 0, 0, 0, 0],
    };

    pub const fn new(d: i64, m: i64, s: i64, t: i64, q: i64) -> Self {
        Self {
            components: [d, m, s, t, q],
        }
    }

    pub fn from_raw(raw: i64) -> Self {
        let sign = if raw < 0 { -1 } else { 1 };
        let mut val = raw.abs();
        let q = val % 60;
        val /= 60;
        let t = val % 60;
        val /= 60;
        let s = val % 60;
        val /= 60;
        let m = val % 60;
        val /= 60;
        let d = val;
        Self {
            components: [d * sign, m * sign, s * sign, t * sign, q * sign],
        }
    }

    pub fn from_int(i: i64) -> Self {
        Self::from_raw(i * Self::SCALE_0)
    }

    pub const fn zero() -> Self {
        Self::ZERO
    }
    pub const fn one() -> Self {
        Self::ONE
    }

    pub fn abs(&self) -> Self {
        Self::from_raw(self.to_raw().abs())
    }

    pub fn to_raw(&self) -> i64 {
        let r = (self.components[0] as i128 * Self::SCALE_0 as i128)
            + (self.components[1] as i128 * Self::SCALE_1 as i128)
            + (self.components[2] as i128 * Self::SCALE_2 as i128)
            + (self.components[3] as i128 * Self::SCALE_3 as i128)
            + (self.components[4] as i128 * Self::SCALE_4 as i128);
        r.clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }

    pub fn to_degrees(&self) -> i64 {
        self.components[0]
    }

    /// Convierte TQ batman-adv (0-255) a SPA normalizado [0,1)
    pub fn from_tq(tq: u8) -> Self {
        // tq/255 en base-60: escalamos a SCALE_0
        let raw = (tq as i128 * Self::SCALE_0 as i128 / 255) as i64;
        Self::from_raw(raw)
    }

    pub fn div_safe(self, other: Self) -> Result<Self, SPAError> {
        let v2 = other.to_raw();
        if v2 == 0 {
            return Err(SPAError::DivisionByZero);
        }
        let v1 = self.to_raw() as i128;
        Ok(Self::from_raw(
            ((v1 * Self::SCALE_0 as i128) / v2 as i128) as i64,
        ))
    }
}

impl Add for SPA {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::from_raw(self.to_raw().wrapping_add(o.to_raw()))
    }
}

impl Sub for SPA {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::from_raw(self.to_raw().wrapping_sub(o.to_raw()))
    }
}

impl Mul for SPA {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        let r = (self.to_raw() as i128 * o.to_raw() as i128) / Self::SCALE_0 as i128;
        Self::from_raw(r as i64)
    }
}

impl Mul<i64> for SPA {
    type Output = Self;
    fn mul(self, s: i64) -> Self {
        Self::from_raw(self.to_raw().wrapping_mul(s))
    }
}

impl Div for SPA {
    type Output = Self;
    fn div(self, o: Self) -> Self {
        self.div_safe(o).expect("SPA División por cero")
    }
}

impl Neg for SPA {
    type Output = Self;
    fn neg(self) -> Self {
        Self::from_raw(-self.to_raw())
    }
}

impl fmt::Display for SPA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = self.abs().components;
        let sign = if self.to_raw() < 0 { "-" } else { "" };
        write!(
            f,
            "S60[{}{:03};{:02},{:02},{:02},{:02}]",
            sign, c[0], c[1], c[2], c[3], c[4]
        )
    }
}

impl fmt::Debug for SPA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

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

const _: () = assert!(
    std::mem::size_of::<HolographicSeedBinary>() == 32,
    "HolographicSeedBinary debe medir exactamente 32 bytes según OPENSPEC-MYC-001"
);

impl HolographicSeedBinary {
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

    pub fn from_lfm(token_id: u32, q: i32, r: i32, phase_sector: u8) -> Self {
        let s = -q - r;
        let normalized = (token_id % 3600) as i64;
        let amp = normalized * SPA::SCALE_0;
        Self::new(token_id, q, r, s, amp, phase_sector, 10, 0x01)
    }

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

    pub fn is_valid_crc(&self) -> bool {
        self.crc16 == self.compute_checksum()
    }

    pub fn is_valid_cubic(&self) -> bool {
        self.coord_q + self.coord_r + self.coord_s == 0
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tq_to_spa_full() {
        let s = SPA::from_tq(255);
        // 255/255 = 1.0 → SPA[001;00,00,00,00]
        assert_eq!(s, SPA::one());
    }

    #[test]
    fn tq_to_spa_zero() {
        let s = SPA::from_tq(0);
        assert_eq!(s, SPA::zero());
    }

    #[test]
    fn add_wraps_correctly() {
        let a = SPA::new(0, 59, 0, 0, 0);
        let b = SPA::new(0, 1, 0, 0, 0);
        let r = a + b;
        assert_eq!(r.components[0], 1);
        assert_eq!(r.components[1], 0);
    }

    #[test]
    fn div_safe_by_zero_returns_error() {
        let a = SPA::one();
        let b = SPA::zero();
        let r = a.div_safe(b);
        assert_eq!(r, Err(SPAError::DivisionByZero));
    }

    #[test]
    #[should_panic(expected = "SPA División por cero")]
    fn div_by_zero_panics() {
        let a = SPA::one();
        let b = SPA::zero();
        let _ = a / b;
    }
}
