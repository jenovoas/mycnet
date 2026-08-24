## Context

Con la especificación `OPENSPEC-MYC-001` archivada, MycNet cuenta con el layout binario de 32 bytes (`HolographicSeedBinary`) y el lector SHM `SentinelFractalBridge`. Este diseño describe cómo el daemon `mycnetd` conecta el puente local con la red mesh L2/L3 (`batman-adv` / WireGuard), cerrando el bucle de sincronización entre máquinas distintas (ej. `laptop` ↔ `fenix`).

## Goals / Non-Goals

**Goals:**
- Implementar el socket UDP no bloqueante en `mycnetd` para emisión y recepción de paquetes de 40 bytes (8B header + 32B semilla).
- Integrar la tarea asíncrona de sondeo armónico a ~41.77 Hz (`YHWH` tick) sobre Tokio en `crates/mycnet-daemon`.
- Propagar automáticamente semillas entrantes hacia el `LiquidLattice` compartido de `mycnet-memory`.
- Mantener la pureza sexagesimal S60 en el cálculo de métricas de red y coherencia.

**Non-Goals:**
- No se implementa retransmisión pesada tipo TCP; las semillas perdidas se compensan por el pulso holográfico periódico y la purga entrópica Salto-17 (68s).

## Decisions

### 1. Datagrama UDP Ligero (40 Bytes) vs Conexión TCP
- **Decisión**: Usar datagramas UDP directos de 40 bytes hacia la subred mesh (`10.10.0.0/24` o WireGuard `10.88.0.0/24`).
- **Razón**: Máxima eficiencia, cero latencia de handshake y acoplamiento directo con la velocidad del Ring 0 eBPF.
- **Alternativas consideradas**: gRPC / HTTP2 (descartado por sobrecarga de serialización y latencia de conexión).

### 2. Sincronización Armónica con Tick YHWH (~23.9 ms)
- **Decisión**: El sondeo de SHM y el despacho de red corren en un intervalo fijo `tokio::time::interval(Duration::from_nanos(23_939_835))`.
- **Razón**: Alinea el ciclo de red con la frecuencia de oscilación natural del lattice (~41.77 Hz), evitando jitter de sondeo.

## Risks / Trade-offs

- **[Riesgo: Congestión de Paquetes en Enlaces Lentos]** → *Mitigación*: Filtrado de emisión: solo se transmiten semillas cuando el TQ del vecino es $\ge 120$ y se usa `cake`/`fq_codel` en la interfaz.
- **[Riesgo: Ataques de Inyección de Semillas]** → *Mitigación*: Validación estricta del encabezado Magic `0x4D594353`, verificación de CRC16 y descarte de coordenadas cúbicas inválidas ($q+r+s \neq 0$).
