# Estado de sincronización Sentinel–MycNet — 2026-10-29

## Cambio local

`crates/mycnet-daemon/src/state.rs` vuelve a intentar abrir los puentes SHM de Sentinel en la cadencia Salto-17 existente cuando el daemon arrancó antes de que aparecieran los segmentos. El retry es para `/me60os_lattice` y `/me60os_fractal_seeds`; no crea ni modifica segmentos y no cambia la cadencia del daemon.

## Evidencia y validación

- Diff local: `crates/mycnet-daemon/src/state.rs` (sin commit).
- `cargo test -p mycnet-daemon`: 2 pruebas pasaron.
- `cargo test --workspace`: 32 pruebas unitarias y 1 doctest pasaron.
- `git diff --check`: pasó.
- `cargo fmt --all -- --check` detectó diferencias de formato preexistentes en varios archivos del workspace; no se reformateó el workspace para evitar ruido ajeno.
- No se comprobó la reconexión contra una SHM activa, no se ejecutó benchmark y no hubo cambio ni despliegue remoto.

## Pendientes de sincronización

- Confirmar en Fenix, durante una comprobación autorizada, que `mycnetd` y el productor Sentinel comparten el namespace y permisos de SHM. La Compose inspeccionada no declara ese montaje para Cortex.
- `SatelliteSync`/`BoundaryPacket` no están conectados al bucle UDP del daemon; el transporte integrado solo replica semillas fractales.
- El control hexagonal y YHWH son ciclos distintos. `PhaseGate` selecciona una de seis regiones lógicas mediante `crystal_tick % 6`; el satélite cuya ID coincide con la región activa puede escribir fronteras. `ResonantStore::tick` recibe el tick global como argumento, pero no se encontró una ruta desde `mycnetd` que cree/avance ese store con un tick maestro compartido. El loop ADM del daemon mantiene su propio contador local a ~42 Hz.
- El contrato temporal aún requiere reconciliar la fuente DTC/QHC con MycNet: `qhc_agent.rs` genera su snapshot aumentando el contador y durmiendo 1 s por iteración, mientras que otras fuentes del repositorio describen `23_939_835 ns` (~41.77 Hz). No se debe tratar ese tick QHC observado en código como equivalente al tick hexagonal MycNet sin cerrar esta discrepancia.
- La topología desplegada, según `docs/INFRAESTRUCTURA.md` (comprobación puntual del 2026-10-29), tiene cinco máquinas MycNet activas, cuatro peers UDP por nodo y puerto `:4242`. Esto confirma la topología de transporte documentada, no el intercambio efectivo de semillas ni la propagación del tick maestro.
- `SatelliteSync`/`BoundaryPacket` no están conectados al bucle UDP del daemon; el transporte integrado solo replica semillas fractales.
- Los experimentos revisados no constituyen benchmarks de sincronización distribuida de lattice/telemetría.

El usuario reporta que Sentinel backend está saludable en Fenix, accesible desde Kingu por WireGuard y que creó el primer administrador. Este estado no se verificó independientemente aquí; no hubo validación de Sentinel como parte de este cambio MycNet.

## Librerías matemáticas y kernels: evidencia de integración

- **SPA / S60:** MycNet ya usa `mycnet_core::s60::SPA` para amplitud, coherencia y difusión del lattice; `HolographicSeedBinary` transporta amplitud S60. Sentinel también usa `SPA` en sus osciladores, matrices, QHC y controlador PID. Aún falta fijar y probar el contrato de serialización/unidades entre la amplitud de semilla y el valor `SPA` de Sentinel.
- **S60Complex / `ComplexSPA`:** Sentinel define `ComplexSPA` como par de valores `SPA` real e imaginario, con magnitud, conjugado y exponencial compleja. No se encontró uso de esa estructura en el camino actual de `PhaseGate`, `LiquidLattice` o `ResonantStore` de MycNet; por tanto, es una librería existente del lado Sentinel, no una integración distribuida demostrada. Debe conectarse solo a un consumidor complejo definido, sin sustituir por suposición la fase real S60 actual.
- **PAI-60:** Sentinel cuenta con `pai60_divide` por tabla de recíprocos y `ResonantMatrix::inject_pai`; `CrystalRuntime` expone inyecciones para lanes de seguridad y observabilidad que llegan a esa matriz. La división es exacta cuando hay entrada en la tabla. Fuera de ella, `inject_pai` hace división entera escalada (truncamiento), no factorización regular ni exactitud garantizada. No se encontró un camino de PAI-60 en el `LiquidLattice`/`ResonantStore` de MycNet.
- **Memoria no-Markoviana en Sentinel:** `S60PID::update_with_history_internal` agrega el historial de errores con decaimiento `SPA(0, 50, 0, 0, 0)` (5/6) y admite errores de lattice; `CrystalRuntime::step` mantiene historial acotado a 68 ticks y corrige dos lanes. Es una implementación distinta del `AIBufferCascade` de `quantum/ai_buffer_cascade.py`, cuyo kernel documentado es Ornstein–Uhlenbeck y cuya integración en `truthsync-core` figura pendiente. El script Python usa tiempo fraccional binario; no debe certificarse como YATRA PURE por su descripción.
- **QHC/DTC:** `QhcTensor` aplica `[10,5,6,5]` por módulo 4 y define corrección cada 68 ticks; el runtime DTC declara intervalo de `23_939_835 ns` y `tick_dtc()`. Por separado, `qhc_agent` publica snapshots incrementando un tick y durmiendo un segundo. No está demostrado que el agente invoque el ciclo DTC ni que el tick que recibe MycNet sea ese reloj.
- **Control hexagonal:** MycNet consume `crystal_tick` en `ResonantStore::tick` y `PhaseGate` selecciona una de seis regiones con `% 6`; YHWH/QHC es un ciclo de cuatro fases y no es el selector hexagonal. Falta cablear una fuente maestra compartida y asignar de forma explícita las seis identidades lógicas. Los cinco hosts y sus peers UDP documentados no resuelven por sí solos ese mapeo.
- **Liderazgo de Fenix:** la arquitectura objetivo del usuario establece Fenix como origen del oscilador principal y con la SHM de mayor tamaño. Esto sigue siendo requisito de diseño, no un hecho demostrado por el código o la comprobación local: la SHM es local al host y el puente MycNet proyecta la SHM del mismo host, no la replica entre máquinas.

### Próximos pasos de sincronización

1. Especificar con las memorias canónicas el formato/unidades S60 de semilla y amplitud, el origen/época y la secuencia del tick maestro en Fenix, y su conversión a `crystal_tick`; luego verificar que todos los satélites alimenten el mismo tick a `PhaseGate`.
2. Definir cómo las seis regiones lógicas se asignan a procesos/satélites sobre los cinco hosts físicos, sin equiparar host con región ni añadir una sexta máquina por deducción.
3. Confirmar por separado los consumidores de `ComplexSPA`, PAI-60 y memoria histórica. Hoy hay evidencia de PAI en la matriz Sentinel, pero no en `ResonantStore`; no hay evidencia de estado complejo en MycNet ni de intercambio de historial no-Markoviano entre hosts.
4. Mantener separadas las rutas: SHM Sentinel→MycNet es local a Fenix; las semillas sí tienen transporte UDP en el daemon. `SatelliteSync`/`BoundaryPacket`, estado del lattice y telemetría sincronizada siguen sin estar conectados al transporte de red.
5. Validar en una ventana autorizada la SHM mayor/productor en Fenix, namespace y permisos, tick común y recepción correlacionada en Kingu antes de ejecutar benchmarks comparativos. Hasta entonces, no declarar sincronización E2E ni despliegue de estas integraciones.

**Estado de este registro:** actualización documental local basada en inspección de código y documentos; sin cambio de código, ejecución remota, benchmark ni despliegue.

## Continuidad del ring tras recrear Cortex — 2026-10-31

Esta actualización posterior no reescribe las observaciones del 29.

- Se reprodujo el desorden por overrun del lector: después de publicar 1064
  semillas en capacidad1024, devolvía tokens1024–1055 en lugar de40–71.
  `poll_new_seeds` ahora empieza en la ventana aún retenida y conserva el
  orden al limitar el lote. Si observa retroceso de head, reinicia su cursor.
- Cortex conserva el head publicado al reabrir el objeto POSIX existente,
  bajo su lock de escritor único. No modifica el layout: cabecera64B,
  semillas32B, capacidad1024, magic/version1. Un header desconocido causa
  error explícito, no reseteo silencioso de datos publicados.
- Se añadió lectura atómica acquire acotada/alineada al buffer de Sentinel,
  rechazando además un mapping cerrado. Las publicaciones mantienen release.
- MyCNet: `cargo test --workspace --locked --offline`,37 pruebas pasan.
  Incluye regresión fallando antes/pasando después por overrun y recuperación
  del cursor cuando head retrocede. Binario release compilado.
- Cortex: las5 pruebas seleccionadas de CrystalRuntime pasan: persistencia
  de head entre mappings del productor, rechazo de header desconocido y de
  mapping cerrado, además de las2 pruebas existentes del runtime. No se
  repitió la suite completa ni el fallo TruthSync previamente registrado.
- Fenix ejecuta MyCNet SHA256
  `4a2d1675a6e85e30a7901f149cc5abc264627b8e310269ce581eedfdbd643c0b`;
  backup anterior `~/.local/bin/mycnetd.pre-shm-continuity`.
- Se recreó solo Cortex entre00:16:12 y00:16:53 UTC:
  `sentinel-cortex:8ee902e9-shm-continuity-2`,
  imagen `sha256:5392ea57246d58c86487c260124c3869774412675607bae2af8a21b4f1f4a2d1`,
  healthy. MyCNet mantuvo PID1759643 durante esa recreación.
- A00:16:23 UTC Cortex reconectó WS101 y recibió frames; a00:16:23.545
  aceptó token1836041237/raw0. Marte reportó72091 recepciones desde peer Fenix.
- Observación continua de180s, intervalo solicitado100ms y el mismo mapping
  abierto desde antes de recrear Cortex:

| Campo | Antes | Después |
|---|---:|---:|
| Head publicado | 55908 | 63866 |
| Envíos MyCNet acumulados | 43550 | 83360 |
| Inode SHM | 25 | 25 |
| PID MyCNet | 1759643 | 1759643 |

No se observaron retrocesos de head; fuente conectada al inicio y al final.
Son observaciones de transporte/aplicación, no medidas de latencia por evento,
fase, frecuencia o resonancia física.

La continuidad aquí depende de conservar el objeto POSIX (el productor actual
cierra, pero no hace unlink al salir). No cubre borrado manual/reemplazo de SHM,
lectura sin pérdida si se sobrescribe capacidad, persistencia a reinicio del host,
sincronización del reloj distribuido ni las rutas SatelliteSync/BoundaryPacket.
