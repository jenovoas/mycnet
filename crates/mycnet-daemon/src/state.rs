// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! Estado compartido del daemon.

use anyhow::Result;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use tokio::sync::{Mutex, broadcast};
use tracing::{info, warn};

use mycnet_core::{ADM, MeshState, PhysicalNode};
use mycnet_core::s60::HolographicSeedBinary;
use mycnet_core::mesh::{get_local_neighbors, mesh_coherence};
use mycnet_core::yhwh::YhwhPhase;
use mycnet_memory::lattice::{Coord, LiquidLattice};
use mycnet_memory::sentinel_bridge::{SentinelBridge, SentinelFractalBridge};

/// Cadencia de refresco desde el lattice fonónico de sentinel:
/// cada 17 ticks = ~405 ms @ 42 Hz = el Salto-17 soberano de Plimpton.
/// No compite con la tasa de sentinel (~41.77 Hz) — solo muestrea.
const SENTINEL_REFRESH_EVERY_TICKS: u64 = 17;

/// Tamaño del lattice fonónico espejo: 3 anillos hexagonales = 37 nodos.
/// Suficiente para interpolar los nodos sentinel sin overflow; los nodos
/// sentinel excedentes se ignoran en `refresh_into_lattice`.
const SENTINEL_LATTICE_RINGS: u32 = 3;

pub struct AppState {
    pub adm:            Mutex<ADM>,
    pub tx:             broadcast::Sender<String>,
    pub hostname:       String,
    pub bat_ip:         String,
    pub lattice:        Mutex<LiquidLattice>,
    pub sentinel:       Mutex<Option<SentinelBridge>>,
    pub fractal_bridge: Mutex<Option<SentinelFractalBridge>>,
    pub metrics_peer: Option<SocketAddr>,
    pub fractal_packets_sent: AtomicU64,
    pub fractal_packets_sent_to_peer: AtomicU64,
    pub fractal_seeds_ingested: AtomicU64,
    pub fractal_seeds_ingested_from_peer: AtomicU64,
}

impl AppState {
    pub fn new(tx: broadcast::Sender<String>, metrics_peer: Option<SocketAddr>) -> Self {
        let mut adm = ADM::new();

        let hex_ring = [
            (1, 0), (1, -1), (0, -1),
            (-1, 0), (-1, 1), (0, 1),
        ];
        adm.add_node(0, 0);
        for (q, r) in hex_ring {
            adm.add_node(q, r);
        }

        let hostname = hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let bat_ip = std::env::var("MYCNET_BAT_IP")
            .unwrap_or_else(|_| "10.10.0.11".into());

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
                    "SentinelBridge no disponible — daemon en modo standalone"
                );
                None
            }
        };

        let fractal_bridge = match SentinelFractalBridge::open() {
            Ok(bridge) => {
                info!("SentinelFractalBridge conectado — ring buffer de semillas /dev/shm/me60os_fractal_seeds activo");
                Some(bridge)
            }
            Err(e) => {
                warn!(error = %e, "SentinelFractalBridge no disponible — sin ring buffer de semillas en /dev/shm");
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
            fractal_bridge: Mutex::new(fractal_bridge),
            metrics_peer,
            fractal_packets_sent: AtomicU64::new(0),
            fractal_packets_sent_to_peer: AtomicU64::new(0),
            fractal_seeds_ingested: AtomicU64::new(0),
            fractal_seeds_ingested_from_peer: AtomicU64::new(0),
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

    /// Registra un datagrama solo después de que `send_to` tuvo éxito.
    pub fn record_fractal_packet_sent(&self, peer: SocketAddr) {
        self.fractal_packets_sent.fetch_add(1, AtomicOrdering::Relaxed);
        if self.metrics_peer == Some(peer) {
            self.fractal_packets_sent_to_peer
                .fetch_add(1, AtomicOrdering::Relaxed);
        }
    }

    /// Inyecta una semilla fractal recibida desde la red mesh remota.
    pub async fn ingest_remote_seed(
        &self,
        seed: HolographicSeedBinary,
        source: SocketAddr,
    ) -> std::result::Result<Coord, &'static str> {
        let mut lattice = self.lattice.lock().await;
        let coord = lattice.inject_fractal_seed(&seed)?;
        self.fractal_seeds_ingested
            .fetch_add(1, AtomicOrdering::Relaxed);
        if self.metrics_peer == Some(source) {
            self.fractal_seeds_ingested_from_peer
                .fetch_add(1, AtomicOrdering::Relaxed);
        }

        let _ = self.tx.send(json!({
            "type": "fractal_seed_ingested",
            "token_id": seed.token_id,
            "coord": [coord.q, coord.r],
            "amplitude_s60": seed.amplitude_s60,
            "phase_sector": seed.phase_sector,
        }).to_string());

        Ok(coord)
    }

    /// Sondea semillas nuevas generadas por Sentinel en SHM para difundirlas a la red mesh.
    pub async fn poll_local_fractal_seeds(&self) -> Vec<HolographicSeedBinary> {
        let mut bridge_guard = self.fractal_bridge.lock().await;
        if let Some(ref mut bridge) = *bridge_guard {
            let seeds = bridge.poll_new_seeds(32);
            if !seeds.is_empty() {
                let mut lattice = self.lattice.lock().await;
                for seed in &seeds {
                    let _ = lattice.inject_fractal_seed(seed);
                }
            }
            seeds
        } else {
            Vec::new()
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
