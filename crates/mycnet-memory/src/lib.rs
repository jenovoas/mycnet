//! # mycnet-memory — Memoria Resonante Distribuida
//!
//! Sistema de almacenamiento distribuido inspirado en redes miceliales.
//! Los datos no se "guardan" estáticamente: se almacenan como patrones
//! de amplitud/fase que fluyen y se auto-reparan a través del lattice.
//!
//! ## Arquitectura basada en experimentos validados
//!
//! - **EXP-009** Liquid Lattice: difusión Von Neumann, 72% retención vs 44% ECC
//! - **EXP-012** Phase Compression: canal dual Amplitud (datos) + Fase (metadatos)
//! - **EXP-015** Rust Benchmark: 16 bytes/nodo, 120M nodos/s
//! - **EXP-028** Penta-Resonance: portales de sincronización (Crystal 41.77 Hz)
//! - **EXP-029** Quantum Scheduler: phase gating — 1/6 red por tick
//!
//! ## Principio Micelial
//!
//! Los datos fluyen como nutrientes en una red de micelio:
//! - No hay nodo maestro, cualquier ruta llega a cualquier dato
//! - Si un satélite cae, sus vecinos absorben su región del lattice
//! - El TQ batman-adv es la fuerza de acoplamiento entre satélites
//! - Phase gating garantiza escrituras sin conflictos ni locks distribuidos

pub mod lattice;
pub mod phase_gate;
pub mod sync;
pub mod storage;

pub use lattice::{LiquidLattice, LatticeNode, DiffusionStep};
pub use phase_gate::{PhaseGate, HexRegion};
pub use sync::{BoundaryPacket, SatelliteSync};
pub use storage::{ResonantStore, StoreResult};
