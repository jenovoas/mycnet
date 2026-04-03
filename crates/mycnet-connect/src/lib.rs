//! # mycnet-connect — Módulo de Conexión para Programadores
//!
//! Este es el ÚNICO módulo que los programadores externos necesitan.
//! Toda la lógica interna (S60, ADM, batman-adv) está encapsulada
//! en `mycnet-core` y `mycnet-daemon`.
//!
//! ## Uso rápido
//!
//! ```rust
//! use mycnet_connect::{MycNetClient, MycNetHandler, MeshEvent, NodeInfo};
//!
//! struct MyApp;
//!
//! impl MycNetHandler for MyApp {
//!     fn on_node_joined(&self, node: NodeInfo) {
//!         println!("Nuevo nodo: {} (coherencia: {})", node.id, node.coherence_s60);
//!     }
//!     fn on_node_left(&self, node_id: &str) {
//!         println!("Nodo caído: {}", node_id);
//!     }
//!     fn on_mesh_event(&self, event: MeshEvent) {
//!         println!("Evento: {:?}", event);
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = MycNetClient::new("http://localhost:7474");
//!     client.connect(MyApp).await.unwrap();
//! }
//! ```

pub mod client;
pub mod types;

pub use client::MycNetClient;
pub use types::{MeshEvent, NodeInfo, MeshSnapshot, NeighborInfo};

/// Trait que los programadores implementan para reaccionar a eventos del mesh.
///
/// Implementar este trait es TODO lo que se necesita para integrarse con MycNet.
pub trait MycNetHandler: Send + Sync + 'static {
    /// Un nuevo nodo se unió a la red (o fue detectado por primera vez).
    fn on_node_joined(&self, node: NodeInfo);

    /// Un nodo dejó de responder o fue desconectado.
    fn on_node_left(&self, node_id: &str);

    /// Evento general del mesh (tick, coherencia, fase YHWH, etc.).
    fn on_mesh_event(&self, event: MeshEvent);
}
