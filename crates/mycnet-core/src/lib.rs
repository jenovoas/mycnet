//! # mycnet-core — Núcleo Privado MycNet
//!
//! IP propietaria: S60 Engine + ADM (Axial Diffusion Model) + Mesh batman-adv.
//! Este crate NO es público. Los programadores externos usan `mycnet-connect`.

pub mod adm;
pub mod mesh;
pub mod s60;
pub mod yhwh;

pub use adm::{AxialCoord, MycNode, ADM};
pub use mesh::{MeshState, Neighbor, PhysicalNode};
pub use s60::SPA;
