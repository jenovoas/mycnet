## 1. Encapsulación y Protocolo de Red Wire

- [ ] 1.1 Definir `FractalMeshPacket` en `crates/mycnet-core/src/mesh.rs` con layout de 40 bytes (8B header + 32B `HolographicSeedBinary`) y serialización determinista.
- [ ] 1.2 Implementar métodos `to_bytes(&self) -> [u8; 40]` y `from_bytes(bytes: &[u8; 40]) -> Result<Self, &'static str>` verificando magic `0x4D594353`.
- [ ] 1.3 Añadir pruebas unitarias de validación y rechazo de paquetes corruptos en `crates/mycnet-core/src/mesh.rs`.

## 2. Emisor y Receptor UDP en Daemon (`mycnetd`)

- [ ] 2.1 Implementar el bucle asíncrono de sondeo armónico en `crates/mycnet-daemon/src/main.rs` con `tokio::time::interval(Duration::from_nanos(23_939_835))`.
- [ ] 2.2 Integrar `SentinelFractalBridge` en `mycnetd` para leer semillas desde `/dev/shm/me60os_fractal_seeds` y despacharlas vía UDP.
- [ ] 2.3 Implementar listener UDP en `mycnetd` para recibir datagramas de semillas e inyectarlos directamente en el `LiquidLattice` compartido de `mycnet-memory`.

## 3. Integración y Validación de Transporte Mesh

- [ ] 3.1 Conectar la actualización de telemetría y métricas SPA al recibir/enviar semillas fractales.
- [ ] 3.2 Probar simulación local de transporte bidireccional (emisor $\leftrightarrow$ receptor) entre dos puertos UDP locales con `cargo test`.
- [ ] 3.3 Validar que todo el pipeline cumple `#![forbid(clippy::float_arithmetic)]` y pasa `cargo test` al 100%.
