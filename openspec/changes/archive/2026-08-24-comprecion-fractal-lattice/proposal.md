## Why

El almacenamiento y sincronización tradicional de estados en redes distribuidas y sistemas cognitivos (Sentinel / MycNet) depende de buffers lineales planos que saturan el bus de memoria y el ancho de banda de red. La compresión fractal sobre Liquid Lattice permite representar y propagar estados complejos mediante una semilla holográfica discreta de 32 bytes, permitiendo que cada nodo del cluster reconstruya el estado exacto en $O(1)$ por difusión fonónica sexagesimal pura ($S60$, $\text{SCALE\_0} = 12\,960\,000$) sin contaminación de punto flotante.

## What Changes

- **Semilla Holográfica Binaria (`HolographicSeedBinary`)**: Estructura de 32 bytes exactos (`#[repr(C)]`) que encapsula token, coordenadas cúbicas hexagonales $(q, r, s)$, amplitud SPA y sector de fase.
- **Inyección y Difusión Fonónica en Liquid Lattice**: Mecanismo de propagación de Von Neumann en malla hexagonal de 37 osciladores, conservando la energía global bajo la modulación YHWH 10-5-6-5.
- **Sincronización Holográfica Zero-Payload**: Motor distribuido que propaga únicamente semillas de 32 bytes en lugar de matrices de nodos completas.
- **Puente Zero-Copy POSIX SHM**: Buffer circular lock-free (`/dev/shm/me60os_fractal_seeds`) para acoplar Sentinel (`me-60os-core`) con MycNet (`mycnet-memory`).

## Capabilities

### New Capabilities
- `memory/fractal-lattice-compression`: Compresión fractal, difusión fonónica discreta S60 y sincronización holográfica zero-payload sobre Liquid Lattice.

### Modified Capabilities
<!-- No modified existing capabilities -->

## Impact

- **MycNet**: `crates/mycnet-memory` (módulos `lattice.rs`, `holographic_sync.rs`, `sentinel_bridge.rs`).
- **Sentinel**: `sentinel/me-60os-core` (módulos `liquid_memory.rs`, `quantum_core.rs`, `neural_memory.rs`, `shm_bridge.rs`).
- **Inter-proceso (IPC)**: Segmento de memoria compartida `/dev/shm/me60os_fractal_seeds`.
- **Rendimiento**: Reducción del ancho de banda de red en sincronización a $\approx 32$ bytes por evento, latencia de intercambio IPC $< 1 \mu\text{s}$.
