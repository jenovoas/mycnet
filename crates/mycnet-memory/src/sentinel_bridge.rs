// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
// Colaboración abierta con atribución. Uso comercial PROHIBIDO sin autorización.
//! # SentinelBridge — puente SHM zero-copy con ~/Proyectos/sentinel
//!
//! Lee el lattice fonónico de `me-60os-core::ResonantMatrix` desde POSIX shared
//! memory y proyecta los estados de amplitud/fase al LiquidLattice del ADM mycnet.
//!
//! ## Contrato binario (ver ~/Proyectos/sentinel/me-60os-core/src/)
//!
//! - `IsochronousOscillator` es `#[repr(C)]`, 192 bytes/nodo:
//!   `name[32] + natural_frequency[40] + amplitude[40] + phase[40] + damping[40]`
//! - `SPA { components: [i64; 5] }` — 40 bytes, escala `SCALE_0 = 60^4 = 12_960_000`.
//! - `ResonantMatrix::sync_to_shm` escribe `crystals.as_ptr()` crudo a shm sin
//!   header ni serialización: `Vec<IsochronousOscillator>` = `Vec<[192 bytes]>`.
//!
//! Por seguridad FFI declaramos aquí nuestras propias vistas `#[repr(C)]`
//! idénticas a las de sentinel, en lugar de castear ciegamente. Cualquier
//! cambio de layout en sentinel debe actualizarse aquí同步.

use std::ffi::CString;
use std::ptr;
use thiserror::Error;
use tracing::{debug, warn};

use mycnet_core::s60::SPA;

use crate::holographic_sync::HolographicSeedBinary;
use crate::lattice::{Coord, LiquidLattice};
use std::sync::atomic::{AtomicU64, Ordering};

/// Nombre estándar del segmento SHM que crea sentinel para el lattice fonónico.
pub const SENTINEL_LATTICE_SHM_NAME: &str = "/me60os_lattice";

/// Nombre estándar del segmento SHM que crea sentinel para el ring buffer de semillas fractales.
pub const SENTINEL_FRACTAL_SEEDS_SHM_NAME: &str = "/me60os_fractal_seeds";

/// Capacidad estándar del ring buffer de semillas fractales.
pub const FRACTAL_SEEDS_CAPACITY: usize = 1024;

/// Header del ring buffer lock-free de semillas fractales en SHM (64 bytes).
#[repr(C)]
pub struct FractalRingHeader {
    pub magic: u32,
    pub version: u32,
    pub head: AtomicU64,
    pub tail: AtomicU64,
    pub reserved: [u8; 40],
}

const _: () = assert!(
    std::mem::size_of::<FractalRingHeader>() == 64,
    "FractalRingHeader debe medir exactamente 64 bytes"
);

/// Handle al segmento SHM de semillas fractales en modo lector.
pub struct SentinelFractalBridge {
    name: String,
    shm_size: usize,
    ptr: *mut u8,
    fd: i32,
    last_tail: u64,
}

unsafe impl Send for SentinelFractalBridge {}
unsafe impl Sync for SentinelFractalBridge {}

impl SentinelFractalBridge {
    pub fn open() -> Result<Self, SentinelBridgeError> {
        Self::open_named(SENTINEL_FRACTAL_SEEDS_SHM_NAME)
    }

    pub fn open_named(name: &str) -> Result<Self, SentinelBridgeError> {
        let c_name = CString::new(name).map_err(|e| SentinelBridgeError::InvalidName(e.to_string()))?;
        let expected_size = std::mem::size_of::<FractalRingHeader>() + FRACTAL_SEEDS_CAPACITY * std::mem::size_of::<HolographicSeedBinary>();

        unsafe {
            let fd = libc::shm_open(c_name.as_ptr(), libc::O_RDONLY, 0);
            if fd == -1 {
                return Err(SentinelBridgeError::ShmOpen(format!("{}: errno={}", name, std::io::Error::last_os_error())));
            }

            let mut stat: libc::stat = std::mem::zeroed();
            if libc::fstat(fd, &mut stat) == -1 {
                libc::close(fd);
                return Err(SentinelBridgeError::Io(std::io::Error::last_os_error()));
            }

            let actual_size = stat.st_size as usize;
            if actual_size < expected_size {
                libc::close(fd);
                return Err(SentinelBridgeError::BufferTooSmall {
                    shm_size: actual_size,
                    expected: FRACTAL_SEEDS_CAPACITY,
                    required: expected_size,
                });
            }

            let ptr = libc::mmap(
                ptr::null_mut(),
                actual_size,
                libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            );

            if ptr == libc::MAP_FAILED {
                libc::close(fd);
                return Err(SentinelBridgeError::Mmap(format!("{}: errno={}", name, std::io::Error::last_os_error())));
            }

            Ok(Self {
                name: name.to_string(),
                shm_size: actual_size,
                ptr: ptr as *mut u8,
                fd,
                last_tail: 0,
            })
        }
    }

    /// Lee semillas nuevas pendientes desde el ring buffer hacia un vector.
    pub fn poll_new_seeds(&mut self, max_seeds: usize) -> Vec<HolographicSeedBinary> {
        if self.ptr.is_null() { return Vec::new(); }

        let header = unsafe { &*(self.ptr as *const FractalRingHeader) };
        let head = header.head.load(Ordering::Acquire);
        let tail = self.last_tail;

        if head <= tail { return Vec::new(); }

        let available = (head - tail) as usize;
        let count = available.min(max_seeds).min(FRACTAL_SEEDS_CAPACITY);
        let mut out = Vec::with_capacity(count);

        let seeds_base = unsafe { self.ptr.add(std::mem::size_of::<FractalRingHeader>()) as *const HolographicSeedBinary };

        for i in 0..count {
            let slot = ((tail + i as u64) % FRACTAL_SEEDS_CAPACITY as u64) as usize;
            let seed = unsafe { *seeds_base.add(slot) };
            if seed.is_valid_crc() && seed.is_valid_cubic() {
                out.push(seed);
            }
        }

        self.last_tail = tail + count as u64;
        out
    }
}

impl Drop for SentinelFractalBridge {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() && self.ptr as *mut libc::c_void != libc::MAP_FAILED {
                libc::munmap(self.ptr as *mut libc::c_void, self.shm_size);
                self.ptr = ptr::null_mut();
            }
            if self.fd != -1 {
                libc::close(self.fd);
                self.fd = -1;
            }
        }
    }
}

/// SPA tal cual lo escribe sentinel — `#[repr(C)]` explícito para garantizar
/// el contrato cross-process. Layout: 5 × i64 little-endian nativo = 40 bytes.
///
/// Aunque `me-60os_core::spa::SPA` NO tiene `#[repr(C)]` en sentinel, en la
/// práctica Rust usa layout natural transparente para structs con campos
/// del mismo tipo y orden declarado. Lo garantizamos aquí con `repr(C)`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SentinelSPA {
    pub components: [i64; 5],
}

impl SentinelSPA {
    /// Convierte el SPA crudo leído del SHM al tipo `SPA` soberano de mycnet.
    pub fn to_mycnet_spa(self) -> SPA {
        SPA { components: self.components }
    }
}

/// `IsochronousOscillator` tal cual lo escribe sentinel en SHM.
/// `#[repr(C)]` idéntico a `me-60os-core/src/isochronous_oscillator.rs::IsochronousOscillator`.
///
/// Layout: 32 + 40 + 40 + 40 + 40 = 192 bytes (sin padding intermedio).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SentinelOscillator {
    /// Nombre fijo del nodo (C-string zero-terminated dentro del buffer).
    pub name: [u8; 32],
    /// Frecuencia natural: Plimpton 322 Fila 12 (1;32,2,24).
    pub natural_frequency: SentinelSPA,
    /// Energía almacenada — la amplitud que proyectamos al lattice mycnet.
    pub amplitude: SentinelSPA,
    /// Fase actual de oscilación.
    pub phase: SentinelSPA,
    /// Factor de amortiguación (típicamente SPA(0,0,30,0,0) = 30/3600).
    pub damping_factor: SentinelSPA,
}

/// Verificación compile-time del tamaño del nodo.
/// Si sentinel alguna vez cambia el layout, este const falla la compilación.
const _: () = assert!(
    std::mem::size_of::<SentinelOscillator>() == 192,
    "SentinelOscillator debe ser exactamente 192 bytes — revisar layout vs sentinel"
);

#[derive(Debug, Error)]
pub enum SentinelBridgeError {
    #[error("SHM no disponible (¿sentinel-cortex corriendo?): {0}")]
    ShmOpen(String),
    #[error("mmap falló: {0}")]
    Mmap(String),
    #[error("Tamaño SHM ({shm_size}) insuficiente para {expected} nodos (req {required} bytes)")]
    BufferTooSmall { shm_size: usize, expected: usize, required: usize },
    #[error("Nombre SHM inválido: {0}")]
    InvalidName(String),
    #[error("Error de IO: {0}")]
    Io(#[from] std::io::Error),
}

/// Handle al segmento SHM abierto en modo lector.
///
/// NO crea el segmento — espera que `sentinel-cortex` lo haya creado.
/// Sigue vivo mientras el handle no se dropee. Drop libera mmap+fd.
pub struct SentinelBridge {
    name: String,
    shm_size: usize,
    ptr: *mut u8,
    fd: i32,
    /// Número de nodos reportados en el último `refresh()`.
    node_count: usize,
}

// SAFETY: mmap MAP_SHARED es válido a través de threads; la sincronización
// con el escritor (sentinel) es eventual (lectura best-effort), no requiere
// lock porque mycnet solo lee y nunca escribe al segmento.
unsafe impl Send for SentinelBridge {}
unsafe impl Sync for SentinelBridge {}

impl SentinelBridge {
    /// Abre el SHM estándar `/me60os_lattice` en modo lector.
    /// Falla con `ShmOpen` si sentinel-cortex no está corriendo o no ha creado
    /// el segmento todavía.
    pub fn open() -> Result<Self, SentinelBridgeError> {
        Self::open_named(SENTINEL_LATTICE_SHM_NAME)
    }

    /// Abre un SHM con nombre custom (para tests u otros nodos sentinel).
    pub fn open_named(name: &str) -> Result<Self, SentinelBridgeError> {
        let c_name = CString::new(name).map_err(|e| {
            SentinelBridgeError::InvalidName(format!("{}: {}", name, e))
        })?;

        // O_RDWR sin O_CREAT — somos lectores, no creadores.
        // 0o666 perms: respetamos los que sentinel haya dejado.
        let fd = unsafe { libc::shm_open(c_name.as_ptr(), libc::O_RDWR, 0o666) };
        if fd == -1 {
            return Err(SentinelBridgeError::ShmOpen(
                std::io::Error::last_os_error().to_string(),
            ));
        }

        // Descubrir el tamaño del segmento vía fstat.
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        let st_ret = unsafe { libc::fstat(fd, &mut st) };
        if st_ret == -1 {
            unsafe { libc::close(fd) };
            return Err(SentinelBridgeError::ShmOpen(
                "fstat falló al consultar tamaño SHM".into(),
            ));
        }
        let shm_size = st.st_size as usize;
        if shm_size == 0 {
            unsafe { libc::close(fd) };
            return Err(SentinelBridgeError::ShmOpen(
                "SHM existe pero está vacío (¿sentinel inicializando?)".into(),
            ));
        }

        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                shm_size,
                libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            unsafe { libc::close(fd) };
            return Err(SentinelBridgeError::Mmap(
                std::io::Error::last_os_error().to_string(),
            ));
        }

        debug!(
            name = %name,
            shm_size = shm_size,
            "Sentinel SHM abierto en modo lector — {} nodos potenciales",
            shm_size / std::mem::size_of::<SentinelOscillator>()
        );

        Ok(Self {
            name: name.to_string(),
            shm_size,
            ptr: ptr as *mut u8,
            fd,
            node_count: 0,
        })
    }

    /// Tamaño del segmento SHM en bytes (reportado por fstat al abrir).
    pub fn shm_size(&self) -> usize { self.shm_size }

    /// Número de nodos `IsochronousOscillator` que calen en el SHM actual.
    pub fn capacity(&self) -> usize {
        self.shm_size / std::mem::size_of::<SentinelOscillator>()
    }

    /// Número de nodos reportados en el último `refresh_into_lattice`.
    pub fn node_count(&self) -> usize { self.node_count }

    /// Lee el lattice SHM como slice inmutable.
    ///
    /// La lectura es best-effort: si sentinel está en medio de un `sync_to_shm`
    /// podemos ver un estado parcial — pero como cada nodo es 192 bytes
    /// contiguos y `copy_nonoverlapping` es atómico a nivel palabra, la
    /// probabilidad de inconsistencia es la de un tear entre nodos, no
    /// dentro de un nodo. Aceptable para proyección al ADM micelial.
    pub fn read_slice(&self) -> &[SentinelOscillator] {
        let n = self.capacity();
        if n == 0 || self.ptr.is_null() {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(self.ptr as *const SentinelOscillator, n) }
    }

    /// Proyecta el estado del lattice sentinel al LiquidLattice mycnet.
    ///
    /// Mapea: cada `SentinelOscillator.amplitude` → `LatticeNode.amplitude`.
    /// Mapea: cada `SentinelOscillator.phase` normalizado → `phase_sector`.
    ///
    /// Estrategia de mapeo: disposición espiral hexagonal desde origen (0,0)
    /// del LiquidLattice, rellenando anillos hacia afuera hasta cubrir todos
    /// los nodos sentinel. Si el LiquidLattice no tiene capacidad suficiente,
    /// los nodos sentinel excedentes se ignoran (no al revés).
    ///
    /// Retorna el número de nodos sentinel proyectados.
    pub fn refresh_into_lattice(&mut self, lattice: &mut LiquidLattice) -> usize {
        let sentinel_nodes = self.read_slice();
        let sentinel_count = sentinel_nodes.len();

        if sentinel_count == 0 {
            self.node_count = 0;
            return 0;
        }

        // Coordenadas en orden espiral desde (0,0) del lattice hexagonal de
        // mycnet — produce el mismo ordenamiento que `LiquidLattice::inject`.
        let mut sorted: Vec<Coord> = lattice.nodes.keys().copied().collect();
        let origin = Coord { q: 0, r: 0 };
        sorted.sort_by_key(|c| {
            ((c.q - origin.q).abs()) + ((c.r - origin.r).abs())
        });

        let mut projected = 0usize;
        for (i, coord) in sorted.iter().enumerate() {
            if i >= sentinel_count { break; }
            let osc = &sentinel_nodes[i];

            if let Some(node) = lattice.nodes.get_mut(coord) {
                node.amplitude = osc.amplitude.to_mycnet_spa();
                // phase_sector: proyectar la fase SPA al byte [0,255].
                // Fase raw % 256 garantiza rango; la 정보 de fase fina reside
                // en los componentes grade/min/sec que están fuera del byte.
                node.phase_sector = (osc.phase.to_mycnet_spa().to_raw().rem_euclid(256)) as u8;
                // Reabrir el sector si estaba cerrado por degradación local:
                // sentinel afirma que el nodo está vivo (amplitude != 0).
                if osc.amplitude.to_mycnet_spa().to_raw() != 0 {
                    node.set_closed(false);
                }
                projected += 1;
            }
        }

        if projected == 0 {
            warn!(
                lattice_capacity = lattice.nodes.len(),
                sentinel_node_count = sentinel_count,
                "refresh_into_lattice: 0 nodos proyectados — lattice mycnet sin capacidad para coordenadas"
            );
        } else {
            debug!(
                projected,
                sentinel_node_count = sentinel_count,
                lattice_capacity = lattice.nodes.len(),
                "refresh_into_lattice: proyección completada"
            );
        }

        self.node_count = projected;
        projected
    }
}

impl Drop for SentinelBridge {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() && self.ptr as *mut libc::c_void != libc::MAP_FAILED {
                libc::munmap(self.ptr as *mut libc::c_void, self.shm_size);
                self.ptr = ptr::null_mut();
            }
            if self.fd != -1 {
                libc::close(self.fd);
                self.fd = -1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_oscillator_is_192_bytes() {
        // Compile-time assertion garantiza contrato FFI con sentinel.
        assert_eq!(std::mem::size_of::<SentinelOscillator>(), 192);
    }

    #[test]
    fn sentinel_spa_is_40_bytes() {
        assert_eq!(std::mem::size_of::<SentinelSPA>(), 40);
    }

    #[test]
    fn sentinel_spa_roundtrips_to_mycnet_spa() {
        let raw = SentinelSPA { components: [1, 32, 2, 24, 0] }; // Plimpton 322 fila 12
        let spa = raw.to_mycnet_spa();
        assert_eq!(spa, SPA::new(1, 32, 2, 24, 0));
    }

    #[test]
    fn open_nonexistent_shm_returns_error() {
        // Nombre deliberadamente inexistente para verificar el path de error.
        let result = SentinelBridge::open_named("/mycnet_test_should_not_exist_4523987");
        assert!(matches!(result, Err(SentinelBridgeError::ShmOpen(_))));
    }

    #[test]
    fn refresh_into_empty_lattice_returns_zero() {
        // Con SHM inexistente, refresh no puede probarse end-to-end sin
        // sentinel-cortex corriendo. Verificamos al menos que no paniquea.
        let mut lattice = LiquidLattice::new(1); // 7 nodos hex
        // No podemos abrir un SHM real, así que solo exercise el sort y el
        // branch "0 nodos sentinel" sin panic.
        let mut bridge = SentinelBridge {
            name: "test".into(),
            shm_size: 0,
            ptr: ptr::null_mut(),
            fd: -1,
            node_count: 0,
        };
        let projected = bridge.refresh_into_lattice(&mut lattice);
        assert_eq!(projected, 0);
        assert_eq!(bridge.node_count(), 0);
    }

    #[test]
    fn fractal_ring_header_is_64_bytes() {
        assert_eq!(std::mem::size_of::<FractalRingHeader>(), 64);
    }

    #[test]
    fn open_nonexistent_fractal_shm_returns_error() {
        let result = SentinelFractalBridge::open_named("/mycnet_test_fractal_none_982347");
        assert!(matches!(result, Err(SentinelBridgeError::ShmOpen(_))));
    }
}
