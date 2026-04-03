//! # mycnet-core — Núcleo Privado MycNet
//!
//! IP propietaria: S60 Engine + ADM (Axial Diffusion Model) + Mesh batman-adv.
//! Este crate NO es público. Los programadores externos usan `mycnet-connect`.

pub mod s60;
pub mod adm;
pub mod mesh;
pub mod yhwh;

pub use s60::SPA;
pub use adm::{ADM, MycNode, AxialCoord};
pub use mesh::{MeshState, Neighbor, PhysicalNode};
