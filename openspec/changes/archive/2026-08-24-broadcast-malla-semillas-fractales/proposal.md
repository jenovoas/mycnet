## Why

Una vez implementada la compresión fractal de 32 bytes y el puente zero-copy en SHM, el siguiente paso crítico es conectar el daemon `mycnetd` para que consuma activamente las semillas fractales generadas por Sentinel y las difunda en tiempo real a través de la red mesh (`batman-adv` / WireGuard / UDP) hacia los nodos remotos (ej. Laptop ↔ Fenix), logrando la sincronización holográfica de estado con cero saturación de red.

## What Changes

- **Bucle de Sondeo Armónico en `mycnetd`**: Tarea asíncrona en Tokio que sondea `SentinelFractalBridge` sincronizada con el pulso temporal YHWH (41.77 Hz / ~23.9 ms).
- **Transporte Mesh de Semillas (`MeshSeedBroadcaster`)**: Emisión y recepción de paquetes UDP compactos conteniendo `HolographicSeedBinary` (32B) hacia todos los vecinos de la malla con TQ $\ge 120$.
- **Recepción e Inyección Automática Remota**: Al recibir un paquete de semilla fractal, el daemon receptor valida el CRC16, comprueba la invariante espacial $q+r+s=0$ y la inyecta en su `LiquidLattice` local.
- **Telemetría y Métricas S60**: Exposición de métricas de semillas transmitidas/recibidas y coherencia global de red en el endpoint de Prometheus / monitor S60.

## Capabilities

### New Capabilities
- `network/fractal-mesh-broadcast`: Emisión, transporte mesh L2/L3 y recepción de semillas holográficas de 32 bytes con sincronización en Liquid Lattice.

### Modified Capabilities
<!-- No modified existing capabilities -->

## Impact

- **`crates/mycnet-daemon`**: `src/main.rs`, `src/state.rs` (nueva tarea de background bridge-to-mesh).
- **`crates/mycnet-core`**: `src/adm.rs`, `src/mesh.rs` (encapsulación y validación de paquetes de semillas).
- **`crates/mycnet-connect`**: Notificación de eventos de semillas recibidas a clientes conectados vía WebSocket.
- **Red**: Tráfico de sincronización acotado a exactamente 32 bytes de carga útil por evento.
