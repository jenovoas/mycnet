//! # SPA — Sexagesimal Fixed-Point Arithmetic (Base-60)
//!
//! Motor matemático propietario. Protocolo Yatra Pure.
//! Zero decimal contamination: solo i64, sin f32/f64.
//!
//! Portado desde ME-60OS (me-60os/src/spa.rs)
//! Autor: Jaime Novoa — Yatra Protocol

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};
use serde::{Deserialize, Serialize};

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
    pub const SCALE_1: i64 = 216_000;    // 60^3
    pub const SCALE_2: i64 = 3_600;      // 60^2
    pub const SCALE_3: i64 = 60;
    pub const SCALE_4: i64 = 1;

    pub const ZERO: SPA = SPA { components: [0; 5] };
    pub const ONE:  SPA = SPA { components: [1, 0, 0, 0, 0] };

    pub const fn new(d: i64, m: i64, s: i64, t: i64, q: i64) -> Self {
        Self { components: [d, m, s, t, q] }
    }

    pub fn from_raw(raw: i64) -> Self {
        let sign = if raw < 0 { -1 } else { 1 };
        let mut val = raw.abs();
        let q = val % 60; val /= 60;
        let t = val % 60; val /= 60;
        let s = val % 60; val /= 60;
        let m = val % 60; val /= 60;
        let d = val;
        Self { components: [d * sign, m * sign, s * sign, t * sign, q * sign] }
    }

    pub fn from_int(i: i64) -> Self { Self::from_raw(i * Self::SCALE_0) }

    pub const fn zero() -> Self { Self::ZERO }
    pub const fn one()  -> Self { Self::ONE }

    pub fn abs(&self) -> Self { Self::from_raw(self.to_raw().abs()) }

    pub fn to_raw(&self) -> i64 {
        let r =
            (self.components[0] as i128 * Self::SCALE_0 as i128)
            + (self.components[1] as i128 * Self::SCALE_1 as i128)
            + (self.components[2] as i128 * Self::SCALE_2 as i128)
            + (self.components[3] as i128 * Self::SCALE_3 as i128)
            + (self.components[4] as i128 * Self::SCALE_4 as i128);
        r.clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }

    pub fn to_degrees(&self) -> i64 { self.components[0] }

    /// Convierte TQ batman-adv (0-255) a SPA normalizado [0,1)
    pub fn from_tq(tq: u8) -> Self {
        // tq/255 en base-60: escalamos a SCALE_0
        let raw = (tq as i128 * Self::SCALE_0 as i128 / 255) as i64;
        Self::from_raw(raw)
    }

    pub fn div_safe(self, other: Self) -> Result<Self, SPAError> {
        let v2 = other.to_raw();
        if v2 == 0 { return Err(SPAError::DivisionByZero); }
        let v1 = self.to_raw() as i128;
        Ok(Self::from_raw(((v1 * Self::SCALE_0 as i128) / v2 as i128) as i64))
    }
}

impl Add for SPA {
    type Output = Self;
    fn add(self, o: Self) -> Self { Self::from_raw(self.to_raw().wrapping_add(o.to_raw())) }
}

impl Sub for SPA {
    type Output = Self;
    fn sub(self, o: Self) -> Self { Self::from_raw(self.to_raw().wrapping_sub(o.to_raw())) }
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
    fn mul(self, s: i64) -> Self { Self::from_raw(self.to_raw().wrapping_mul(s)) }
}

impl Div for SPA {
    type Output = Self;
    fn div(self, o: Self) -> Self {
        self.div_safe(o).expect("SPA División por cero")
    }
}

impl Neg for SPA {
    type Output = Self;
    fn neg(self) -> Self { Self::from_raw(-self.to_raw()) }
}

impl fmt::Display for SPA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = self.abs().components;
        let sign = if self.to_raw() < 0 { "-" } else { "" };
        write!(f, "S60[{}{:03};{:02},{:02},{:02},{:02}]", sign, c[0], c[1], c[2], c[3], c[4])
    }
}

impl fmt::Debug for SPA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self) }
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
}
