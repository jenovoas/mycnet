// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! Estado compartido del daemon.

use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use serde_json::json;
use anyhow::Result;
use tracing::{info, warn};

use mycnet_core::{ADM, MeshState, PhysicalNode};
use mycnet_core::mesh::{get_local_neighbors, mesh_coherence};
use mycnet_core::yhwh::YhwhPhase;
use mycnet_memory::lattice::LiquidLattice;
use mycnet_memory::sentinel_bridge::SentinelBridge;

/// Cadencia de refresco desde el lattice fonónico de sentinel:
/// cada 17 ticks = ~405 ms @ 42 Hz = el Salto-17 soberano de Plimpton.
/// No compite con la tasa de sentinel (~41.77 Hz) — solo muestrea.
const SENTINEL_REFRESH_EVERY_TICKS: u64 = 17;

/// Tamaño del lattice fonónico espejo: 3 anillos hexagonales = 37 nodos.
/// Suficiente para interpolar los nodos sentinel sin overflow; los nodos
/// sentinel excedentes se ignoran en `refresh_into_lattice`.
const SENTINEL_LATTICE_RINGS: u32 = 3;

pub struct AppState {
    pub adm:      Mutex<ADM>,
    pub tx:       broadcast::Sender<String>,
    pub hostname: String,
    pub bat_ip:   String,
    /// Lattice fonónico espejo del lattice sentinel (mycnet-memory).
    /// Se actualiza cada 17 ticks vía SentinelBridge. Distinto del ADM:
    /// el ADM captura topología batman-adv; el lattice captura resonancia fonónica.
    pub lattice:  Mutex<LiquidLattice>,
    /// Puente SHM zero-copy con ~/Proyectos/sentinel (lattice fonónico).
    /// None si sentinel-cortex no está corriendo o el SHM no existe.
    pub sentinel: Mutex<Option<SentinelBridge>>,
}

impl AppState {
    pub fn new(tx: broadcast::Sender<String>) -> Self {
        let mut adm = ADM::new();

        // Red hexagonal de 6 nodos (topología estándar MycNet)
        // Anillo hexagonal alrededor del origen
        let hex_ring = [
            (1, 0), (1, -1), (0, -1),
            (-1, 0), (-1, 1), (0, 1),
        ];
        adm.add_node(0, 0); // nodo central
        for (q, r) in hex_ring {
            adm.add_node(q, r);
        }

        let hostname = hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let bat_ip = std::env::var("MYCNET_BAT_IP")
            .unwrap_or_else(|_| "10.10.0.11".into());

        // Puente SHM con sentinel — opcional, no bloqueante al arranque.
        // Si sentinel-cortex no corre, el daemon opera en modo standalone
        // (ADM alimenta solo desde batctl local).
        let sentinel = match SentinelBridge::open() {
            Ok(bridge) => {
                info!(
                    shm_size = bridge.shm_size(),
                    capacity = bridge.capacity(),
                    "SentinelBridge conectado — lattice fonónico proyectado al ADM"
                );
                Some(bridge)
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "SentinelBridge no disponible — daemon en modo standalone (batctl → ADM sin espejo fonónico). \
                     ¿sentinel-cortex corriendo y con SHM /me60os_lattice inicializado?"
                );
                None
            }
        };

        Self {
            adm: Mutex::new(adm),
            tx,
            hostname,
            bat_ip,
            lattice: Mutex::new(LiquidLattice::new(SENTINEL_LATTICE_RINGS)),
            sentinel: Mutex::new(sentinel),
        }
    }

    /// Tick principal: lee batctl → actualiza ADM → broadcast.
    /// Cada 17 ticks (Salto-17): refresca proyección del lattice sentinel al ADM.
    pub async fn tick(&self) -> Result<()> {
        let phase = YhwhPhase::current();

        // Leer vecinos reales de batman-adv
        let neighbors = match get_local_neighbors() {
            Ok(n) => n,
            Err(e) => {
                // Sin batctl disponible (dev mode): simular
                tracing::debug!("batctl no disponible ({}), modo simulación", e);
                Vec::new()
            }
        };

        let coherence = mesh_coherence(&neighbors);

        // Actualizar ADM con TQ de vecinos reales + proyección opcional sentinel
        {
            let mut adm = self.adm.lock().await;

            // Inyectar TQ de cada vecino en su nodo ADM correspondiente
            for (i, neighbor) in neighbors.iter().enumerate() {
                let ring_positions = [
                    (1i64, 0i64), (1, -1), (0, -1),
                    (-1, 0), (-1, 1), (0, 1),
                ];
                if let Some(&(q, r)) = ring_positions.get(i) {
                    adm.inject_tq(q, r, neighbor.tq);
                }
            }

            // Proyección Salto-17: cada 17 ticks, leer el lattice fonónico
            // de sentinel y actualizar el lattice espejo de mycnet.
            // El lattice fonónico refleja la resonancia bio-mechanical de
            // sentinel (acoplada a eBPF Ring-0). El ADM micelial refleja
            // batctl — son dos capas que se complementan, no compiten.
            let tick_now = adm.tick_count;
            let should_refresh = tick_now > 0
                && tick_now % SENTINEL_REFRESH_EVERY_TICKS == 0;

            let mut sentinel_projected = 0usize;
            if should_refresh {
                let mut sentinel_guard = self.sentinel.lock().await;
                if let Some(ref mut bridge) = *sentinel_guard {
                    let mut lattice_guard = self.lattice.lock().await;
                    sentinel_projected = bridge.refresh_into_lattice(&mut lattice_guard);
                    if sentinel_projected > 0 {
                        tracing::debug!(
                            tick = tick_now,
                            sentinel_projected,
                            "Proyección Salto-17 desde sentinel completada"
                        );
                    }
                }
            }

            adm.tick();

            // Snapshot del lattice fonónico para el broadcast
            let lattice_guard = self.lattice.lock().await;
            let lattice_snapshot = lattice_guard.snapshot();
            let lattice_coherence = lattice_guard.global_coherence();
            let sentinel_connected = self.sentinel.lock().await.is_some();
            drop(lattice_guard);

            // Broadcast snapshot
            let snapshot = json!({
                "type":       "mesh_tick",
                "tick":       adm.tick_count,
                "phase":      phase.name(),
                "coherence":  coherence.to_string(),
                "neighbors":  neighbors.len(),
                "nodes":      adm.snapshot(),
                "sentinel": {
                    "connected":          sentinel_connected,
                    "projected_last":     sentinel_projected,
                    "lattice_coherence":  lattice_coherence.to_string(),
                    "lattice_nodes":      lattice_snapshot.len(),
                    "lattice":            lattice_snapshot,
                },
            });

            let _ = self.tx.send(snapshot.to_string());
        }

        Ok(())
    }

    pub async fn get_mesh_state(&self) -> MeshState {
        let neighbors = get_local_neighbors().unwrap_or_default();
        let coherence = mesh_coherence(&neighbors);

        MeshState {
            nodes: vec![PhysicalNode {
                id:              self.hostname.clone(),
                bat_ip:          self.bat_ip.clone(),
                mesh_coherence:  coherence,
                neighbors,
            }],
            ts_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

// Pequeña dependencia para hostname
mod hostname {
    pub fn get() -> Result<std::ffi::OsString, ()> {
        std::process::Command::new("hostname")
            .output()
            .map(|o| std::ffi::OsString::from(
                String::from_utf8_lossy(&o.stdout).trim().to_string()
            ))
            .map_err(|_| ())
    }
}
