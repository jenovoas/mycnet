## Purpose

Define el protocolo de transporte, difusión por red mesh y recepción de semillas fractales de 32 bytes entre nodos MycNet, sincronizando los estados de Liquid Lattice a través de enlaces WireGuard/batman-adv con latencia determinista.

## ADDED Requirements

### Requirement: UDP Fractal Seed Packet Wire Protocol
El sistema SHALL encapsular la semilla fractal `HolographicSeedBinary` de 32 bytes dentro de un datagrama UDP compacto precedido por un encabezado de red de 8 bytes (`magic: 0x4D594353` ("MYCS") + `protocol_version: u16` + `sequence_id: u16`).

#### Scenario: Valid packet wire layout
- **WHEN** el daemon emite una semilla fractal hacia la red mesh
- **THEN** el datagrama UDP resultante mide exactamente 40 bytes (8B header + 32B payload).

### Requirement: Harmonic Shm Polling Loop
El daemon `mycnetd` SHALL sondear el ring buffer `/dev/shm/me60os_fractal_seeds` en sincronía con el reloj orgánico YHWH (~41.77 Hz / cada ~23.9 ms) para extraer y transmitir cualquier nueva semilla generada por Sentinel.

#### Scenario: Real-time seed extraction and broadcast
- **WHEN** Sentinel deposita una nueva semilla en el buffer SHM
- **THEN** el daemon `mycnetd` la extrae en el siguiente ciclo armónico y la transmite por la red mesh a todos los nodos vecinos.

### Requirement: Remote Lattice Ingestion on Packet Receipt
El daemon `mycnetd` SHALL escuchar en el puerto UDP configurado (por defecto 4242) y, al recibir un paquete válido, inyectar inmediatamente la semilla en su `LiquidLattice` local.

#### Scenario: Autonomous local lattice synchronization
- **WHEN** un nodo remoto (ej. Fenix) recibe un paquete de semilla emitido por otro nodo (ej. Laptop)
- **THEN** valida el CRC16, inyecta la semilla en el nodo hexagonal correspondiente y actualiza la coherencia global de la malla.

### Requirement: Corrupted Seed Packet Rejection
El sistema SHALL descartar de forma silenciosa y registrar métricas de error para cualquier datagrama que presente CRC16 corrupto, encabezado inválido o violación de la invariante cúbica $q + r + s \neq 0$.

#### Scenario: Dropping corrupted transmission
- **WHEN** un paquete UDP llega con bits alterados en el payload
- **THEN** la verificación de CRC16 falla, el paquete se descarta sin modificar el `LiquidLattice` y se incrementa el contador de paquetes descartados.
