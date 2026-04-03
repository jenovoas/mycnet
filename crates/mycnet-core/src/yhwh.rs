//! # YHWH Modulation — Protocolo 10-5-6-5
//!
//! Modulación armónica para rebalanceo adaptativo de la red.
//! El patrón (10,5,6,5) divide el ciclo en 4 fases de 6 horas.
//! En cada fase se ajustan los parámetros de backfill/AQM.
//!
//! Autor: Jaime Novoa — Yatra Protocol

use serde::{Deserialize, Serialize};

/// Patrón YHWH: (10, 5, 6, 5) — 4 fases de 6h = 24h
pub const YHWH_PATTERN: [u8; 4] = [10, 5, 6, 5];

/// Fase actual del ciclo YHWH.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum YhwhPhase {
    /// Fase 0 (00-06h): Factor 10 — alta actividad
    Yod,
    /// Fase 1 (06-12h): Factor 5  — media
    He,
    /// Fase 2 (12-18h): Factor 6  — convergencia
    Vav,
    /// Fase 3 (18-24h): Factor 5  — reposo
    HeFinal,
}

impl YhwhPhase {
    /// Calcula la fase según la hora UTC actual.
    pub fn current() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let hour = (secs / 3600) % 24;
        match hour / 6 {
            0 => YhwhPhase::Yod,
            1 => YhwhPhase::He,
            2 => YhwhPhase::Vav,
            _ => YhwhPhase::HeFinal,
        }
    }

    pub fn factor(self) -> u8 {
        YHWH_PATTERN[self as usize]
    }

    /// Parámetros de AQM para batman-adv según la fase.
    pub fn aqm_params(self) -> AqmParams {
        match self.factor() {
            f if f >= 10 => AqmParams { backfills: 2, recovery_sleep_ms: 50 },
            f if f >= 6  => AqmParams { backfills: 1, recovery_sleep_ms: 100 },
            _            => AqmParams { backfills: 1, recovery_sleep_ms: 200 },
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            YhwhPhase::Yod      => "YOD (10)",
            YhwhPhase::He       => "HE (5)",
            YhwhPhase::Vav      => "VAV (6)",
            YhwhPhase::HeFinal  => "HE_FINAL (5)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AqmParams {
    pub backfills:         u8,
    pub recovery_sleep_ms: u32,
}
