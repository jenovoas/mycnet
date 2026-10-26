// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! # mycnet-core — Núcleo Privado MycNet
//!
//! IP propietaria: S60 Engine + ADM (Axial Diffusion Model) + Mesh batman-adv.
//! Este crate NO es público. Los programadores externos usan `mycnet-connect`.

pub mod adm;
pub mod mesh;
pub mod s60;
pub mod yhwh;

pub use s60::{SPA, HolographicSeedBinary, compute_crc16};
pub use adm::{ADM, MycNode, AxialCoord};
pub use mesh::{MeshState, Neighbor, PhysicalNode, FractalMeshPacket, FRACTAL_PACKET_MAGIC, FRACTAL_PROTOCOL_VERSION};
