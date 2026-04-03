//! # PhaseGate — Control de Fase Hexagonal
//!
//! Phase gating: solo 1/6 de la red escribe por tick.
//! Cada satélite gestiona una región hexagonal del lattice.
//!
//! ## Por qué elimina conflictos de escritura sin locks
//!
//! El lattice tiene 6 regiones (coord.hex_region() → 0-5).
//! En el tick T, solo la región `T % 6` puede escribir.
//! Nunca hay dos satélites escribiendo la misma región simultáneamente.
//! → Sin mutexes distribuidos. Sin coordinación de quórum.
//!
//! El Crystal Clock de Sentinel (41.77 Hz) es el árbitro global:
//! todos los satélites usan el mismo tick_count → misma región activa.
//!
//! Referencia: EXP-029 (Quantum Scheduler), redes_micelio_hexagonal.md §4.3

use serde::{Deserialize, Serialize};

/// Las 6 regiones hexagonales del lattice.
/// Cada región corresponde a una dirección axial del hexágono.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HexRegion {
    R0 = 0, R1 = 1, R2 = 2,
    R3 = 3, R4 = 4, R5 = 5,
}

impl HexRegion {
    pub fn from_tick(tick: u64) -> Self {
        match tick % 6 {
            0 => HexRegion::R0, 1 => HexRegion::R1, 2 => HexRegion::R2,
            3 => HexRegion::R3, 4 => HexRegion::R4, _ => HexRegion::R5,
        }
    }

    pub fn as_u8(self) -> u8 { self as u8 }
}

/// Controlador de fase para un satélite.
///
/// Decide en cada tick si el satélite puede escribir a sus vecinos
/// y qué región del lattice procesa.
pub struct PhaseGate {
    /// ID del satélite (0-5 en topología estándar de 6 satélites)
    pub satellite_id: u8,
    /// Tick global sincronizado con Crystal Clock
    pub crystal_tick: u64,
    /// YHWH factor de la fase actual (controla intensidad de difusión)
    pub yhwh_factor:  u8,
}

impl PhaseGate {
    pub fn new(satellite_id: u8) -> Self {
        Self { satellite_id, crystal_tick: 0, yhwh_factor: 10 }
    }

    /// Avanza el tick y retorna la región activa.
    /// Llamar una vez por tick del Crystal Clock (41.77 Hz).
    pub fn advance(&mut self, new_tick: u64, yhwh_factor: u8) -> GateState {
        self.crystal_tick = new_tick;
        self.yhwh_factor  = yhwh_factor;

        let active_region = HexRegion::from_tick(new_tick);

        // El satélite "posee" la región de su ID
        // Solo puede difundir boundaries cuando su región está activa
        let can_write_boundary = active_region.as_u8() == self.satellite_id;

        // Intensidad de difusión basada en YHWH
        // Factor 10 (YOD) = máxima difusión, Factor 5 (HE) = media
        let diffusion_strength = match yhwh_factor {
            f if f >= 10 => DiffusionStrength::High,
            f if f >= 6  => DiffusionStrength::Medium,
            _            => DiffusionStrength::Low,
        };

        GateState {
            active_region,
            can_write_boundary,
            diffusion_strength,
            tick: new_tick,
        }
    }

    /// ¿Puede este satélite procesar el nodo en la coordenada dada?
    pub fn can_process(&self, hex_region: u8) -> bool {
        HexRegion::from_tick(self.crystal_tick).as_u8() == hex_region
    }
}

/// Estado del gate en un tick determinado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateState {
    pub active_region:     HexRegion,
    pub can_write_boundary: bool,
    pub diffusion_strength: DiffusionStrength,
    pub tick:              u64,
}

/// Intensidad de difusión mapeada desde YHWH factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffusionStrength {
    /// YHWH YOD (10): difusión máxima, buffers abiertos
    High,
    /// YHWH VAV (6): difusión media, convergencia
    Medium,
    /// YHWH HE (5): difusión baja, reposo/purga
    Low,
}

impl DiffusionStrength {
    /// Peso de acoplamiento (0-255) para difusión de boundary.
    /// Multiplica el TQ del enlace batman-adv.
    pub fn coupling_weight(self) -> u8 {
        match self {
            DiffusionStrength::High   => 255,
            DiffusionStrength::Medium => 160,
            DiffusionStrength::Low    => 80,
        }
    }
}
