//! Tipos públicos del módulo de conexión.

use serde::{Deserialize, Serialize};

/// Información de un nodo del mesh visible externamente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Identificador del nodo (hostname o MAC)
    pub id: String,
    /// IP en la red bat0
    pub bat_ip: String,
    /// Coherencia del nodo en notación S60 (string, ej: "S60[000;45,00,00,00]")
    pub coherence_s60: String,
    /// Número de vecinos activos
    pub neighbor_count: usize,
}

/// Información de un vecino directo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborInfo {
    pub mac:          String,
    pub iface:        String,
    /// TQ crudo (0-255)
    pub tq:           u8,
    /// TQ en notación S60
    pub tq_s60:       String,
    pub last_seen_ms: u64,
}

/// Snapshot completo del estado del mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshSnapshot {
    pub tick:       u64,
    pub phase:      String,
    pub coherence:  String,
    pub nodes:      Vec<NodeInfo>,
    pub ts_unix:    u64,
}

/// Eventos que el mesh puede emitir.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MeshEvent {
    /// Tick del loop isocrono con estado actual
    Tick {
        tick:      u64,
        phase:     String,
        coherence: String,
        neighbors: usize,
    },
    /// Cambio de fase YHWH (0-3)
    PhaseChange {
        old_phase: String,
        new_phase: String,
        factor:    u8,
    },
    /// Coherencia del mesh por debajo del umbral crítico
    LowCoherence {
        coherence: String,
        threshold: String,
    },
    /// Evento no reconocido (extensibilidad)
    Unknown(serde_json::Value),
}
