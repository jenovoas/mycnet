# memory/fractal-lattice-compression Specification

## Purpose
Proporciona compresión fractal, difusión fonónica sexagesimal y reconstrucción holográfica en tiempo constante O(1) sobre una malla hexagonal resonante (Liquid Lattice), permitiendo la sincronización de estado distribuido con cero payload de red y almacenamiento libre de fragmentación.

## Requirements

### Requirement: Binary Holographic Seed Layout
El sistema SHALL estructurar la semilla holográfica (`HolographicSeedBinary`) en un layout binario exacto de 32 bytes con alineación `#[repr(C)]` little-endian sin campos de relleno indeterminados.

#### Scenario: Exact 32 bytes binary footprint
- **WHEN** se evalúa el tamaño en memoria de la estructura `HolographicSeedBinary`
- **THEN** `std::mem::size_of::<HolographicSeedBinary>()` DEBE ser exactamente 32 bytes.

#### Scenario: Serialization and checksum validation
- **WHEN** se genera una semilla con `token_id`, `amplitude_s60` y coordenadas $(q, r, s)$
- **THEN** el campo `crc16` DEBE coincidir con el cálculo CRC16-CCITT de los primeros 28 bytes de la estructura.

### Requirement: Hexagonal Cubic Coordinate Invariant
El sistema SHALL validar que toda coordenada de inyección en el Liquid Lattice cumpla la restricción espacial de coordenadas cúbicas $q + r + s = 0$.

#### Scenario: Valid cubic coordinate injection
- **WHEN** se inyecta una semilla con $q = 1, r = 0, s = -1$
- **THEN** el validador espacial acepta la coordenada y mapea la energía al oscilador correspondiente en el anillo concéntrico.

#### Scenario: Invalid cubic coordinate rejection
- **WHEN** se intenta inyectar una semilla con $q = 1, r = 1, s = 0$ ($q + r + s \neq 0$)
- **THEN** el sistema rechaza la operación con error de invariante espacial.

### Requirement: Phononic Diffusion Energy Conservation
El sistema SHALL propagar la energía en el Liquid Lattice mediante difusión de Von Neumann en base-60 exacta ($\text{SCALE\_0} = 12\,960\,000$), conservando la suma escalar de energía global $\sum |A_i|$ sin disipación entrópica.

#### Scenario: Multi-tick diffusion energy conservation
- **WHEN** se aplican 68 ticks de difusión fonónica tras una inyección focalizada
- **THEN** la diferencia entre la energía total inicial y final DEBE ser estrictamente menor a $1 / 60^4$.

### Requirement: Zero-Copy Shared Memory IPC Protocol
El sistema SHALL exponer un ring buffer circular lock-free en `/dev/shm/me60os_fractal_seeds` sincronizado con atómicos `AtomicU64` para transferir semillas entre Sentinel (`me-60os-core`) y MycNet (`mycnet-memory`).

#### Scenario: Zero-copy cross-process exchange
- **WHEN** Sentinel escribe una semilla de 32 bytes en el índice de escritura `head`
- **THEN** MycNet lee la semilla en el índice `tail` con latencia sub-microsegundo sin copias intermedias de heap.

### Requirement: Pure Sexagesimal S60 Floating-Point Exclusion
El sistema SHALL prohibir terminantemente el uso de tipos o literales de punto flotante (`f32`, `f64`) en todo el pipeline de compresión fractal y evolución del Liquid Lattice, cumpliendo el Protocolo YATRA.

#### Scenario: Static check against floats
- **WHEN** se compila el workspace con `#![forbid(clippy::float_arithmetic)]`
- **THEN** la compilación pasa sin advertencias ni errores de aritmética decimal.
