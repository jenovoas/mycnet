# MYKV v1 — replicación de metadatos KV

Implementación integrada en `main`, fuente MyCNet `e8cb1ad` (merge publicado el 2026-11-11). Este documento describe el contrato del daemon, no un despliegue. La arquitectura y límites de alcance se anclan en el vault `docs/AGENT_BRAIN.md`: runtime Rust, aritmética SPA y P0.2/P0.3 en pausa. No se inspeccionó producción para esta documentación.

## Transporte y formato

MYKV comparte el socket UDP de MyCNet: `MYCNET_UDP_BIND`, por defecto `0.0.0.0:4242`. Es una vía independiente del ring SHM y del tráfico fractal legacy de **40 bytes**: no modifica su layout, tick, cadencia ni ventanas OU. No se publica KV escribiendo ese ring.

Cada datagrama mide exactamente `80 + key_len` bytes: entre 81 y 592. Los enteros multibyte son little-endian; la longitud cuenta **bytes UTF-8**, no caracteres.

| Bytes inclusivos | Contenido |
|---|---|
| 0–3 | Magic ASCII `MYKV` |
| 4 | Tipo: Publish=1, Replicate=2, Ack=3, Committed=4 |
| 5 | Versión 1 |
| 6–7 | `key_len`: u16, de 1 a 512 |
| 8–15 | `timestamp_ns`: u64, timestamp original del store |
| 16–47 | `HolographicSeedBinary`: 32 bytes originales, con CRC existente |
| 48–79 | Hash BLAKE3 de los bytes originales del payload: 32 bytes |
| 80–fin | Clave UTF-8 completa |

Todos los tipos ecoan la identidad completa `(key, timestamp_ns, seed_bytes, payload_hash)`. Un ACK con cualquier campo distinto no confirma la publicación. Se rechazan tamaños inconsistentes, bytes adicionales, magic/versión/tipo inválidos, UTF-8 inválido, claves no permitidas y semillas inválidas (CRC y coordenadas cúbicas/rangos). Una semilla que no pueda aplicarse al lattice tampoco autoriza ACK.

## Tipos y autorización

| Tipo | Origen admitido por el daemon | Efecto |
|---|---|---|
| Publish | IP loopback | Aplicación local y réplica a todos los peers configurados |
| Replicate | Coincidencia exacta de IP **y puerto** en `MYCNET_PEERS` | Aplicación LWW y Ack al emisor si se acepta |
| Ack | Coincidencia exacta de IP **y puerto** en `MYCNET_PEERS` | Confirma solo esa identidad y ese peer pendiente |
| Committed | Ninguno como entrada | Respuesta del daemon al último productor local de esa identidad |

Solo se admiten claves con prefijo literal `crystal-` o `portal-metrics-`, respetando el límite de 512 bytes. CRC y allowlist de direcciones no son autenticación criptográfica: la frontera privada sigue siendo WireGuard/firewall. No hay protocolo público autenticado nuevo.

## Entrega, ACK y límites

El daemon admite **256 identidades pendientes globales** (por tanto, como máximo 256 por peer), no 256 independientes por peer. Cada identidad guarda un único remitente local: otro Publish idéntico actualiza ese destinatario, sin acumular waiters ni reinyectar el estado.

- Tras aplicar localmente, envía Replicate a todos los peers configurados y reintenta cada **100 ms** solo hacia los que faltan. No renueva timestamp ni identidad; un fallo de envío conserva el pendiente.
- El receptor emite Ack después de aceptar LWW: versión aplicada, duplicada o superada. Un error de capacidad o aplicación no emite Ack.
- Committed se emite únicamente al reunir Ack de **todos** los peers configurados. No es quorum parcial. Sin peers, Publish no se aplica ni se confirma.
- Con 256 pendientes, otra identidad no se almacena ni confirma; el productor conserva y reintenta la misma publicación. No hay expiración del pendiente por timeout del productor.
- Si se pierde Committed, otro Publish idéntico vuelve a reunir los ACK de todos los peers. No existe caché ilimitada de confirmaciones.
- Timeout del productor significa **resultado incierto**: puede haber réplica parcial; no significa ausencia de réplica ni éxito.

El store exacto admite **4096 claves**, sin eviction que olvide la versión LWW. Lleno, rechaza claves nuevas, pero permite duplicados y actualizaciones válidas de claves existentes. Metadatos, pendientes y ACK son **volátiles**: no se promete durabilidad frente a reinicio ni recuperación automática del estado perdido.

## Conflictos y exactitud

Para una misma clave gana el máximo lexicográfico de `(timestamp_ns, payload_hash, seed_bytes)`: primero u64, después bytes del hash y finalmente bytes de semilla. Esto resuelve empates determinísticamente independientemente del orden de recepción. Un duplicado o una versión superada no reemplaza la ganadora ni reinyecta energía, aunque el lattice haya disipado.

El timestamp es el original de la escritura, no el del reintento. Los ticks o secuencias UDP locales no son un reloj causal global. Un reloj de pared adelantado puede dominar versiones; no se garantiza orden causal entre nodos.

La semilla y el hash se conservan **byte a byte**, separados de `put`/`put_timestamped` legacy y su proyección con pérdida. Conservar metadatos exactos no acredita fidelidad ni disponibilidad del payload remoto.

## Consulta HTTP local

`GET /api/v1/kv-metadata?key=<clave UTF-8 escapada>` consulta únicamente el store local:

- Clave permitida presente: HTTP 200 con `key`, `present:true`, `timestamp_ns`, `seed_bytes` (array de 32 enteros), `payload_hash` (array de 32 enteros) y `payload_available:false`.
- Clave permitida ausente: HTTP 200 con `key`, `present:false` y `payload_available:false`; no incluye versión, semilla ni hash.
- Clave fuera de allowlist o del límite: HTTP 400 con `error: "clave fuera de whitelist"`.

No incluye un campo `payload`, no consulta otros nodos y no implementa recall remoto. No es compatible como sustituto de una API de lectura del payload original. El endpoint debug legacy `GET /api/v1/kv/:token` consulta presencia por token u32 en el índice legacy; no prueba presencia de la clave MYKV ni entrega de todos sus peers. `/health` tampoco acredita ese commit.

## Evidencia histórica — 2026-11-11

Registro de la fase anterior, no pruebas nuevas de esta documentación:

- Sentinel `liquid_memory::tests`: 12/12; MyCNet daemon: 14/14; `storage::kv_tests`: 5/5; `s60::tests`: 6/6; `lattice::tests`: 7/7. Storage legacy: 4/4 en la primera tanda. Builds y Clippy estricto aprobados.
- Smoke con tres procesos, binarios reales y UDP loopback: una réplica y un ACK descartados se recuperaron; Committed inicial en **163 ms**. Metadatos idénticos en los tres nodos; empate temporal, reordenamiento y duplicados convergentes; origen no configurado rechazado.
- Peer ausente produjo timeout sin falso Committed, aunque otro peer ya había aplicado la réplica. Reiniciar el peer aislado permitió completar la entrega pendiente; no prueba recuperación durable tras reiniciar el daemon que guarda los pendientes.
- El primer smoke encontró overflow de sumas i64 de coherencia. Se corrigieron medias globales, regionales y de vecinos acumulando en i128 antes de dividir; `SPA::from_raw` usa cociente/resto con signo y soporta `i64::MIN` sin `abs()`. Regresiones de extremos y smoke posterior aprobados.
- Los procesos de prueba terminaron. Sin despliegue, pausa de Cortex, cambios de servicios/firewall ni medición nueva en la malla real.

Trazabilidad: vault `HANDOFF.md`, sección «Memoria KV distribuida — implementación, pruebas y merge — 2026-11-11»; Sentinel integrado `1f1b6b56`, MyCNet integrado `e8cb1ad`.

## Pendientes y fuentes

El cambio central `memoria-kv-distribuida` permanece **9/12**, sin archivar: 3.2 requiere medición en la malla real tras habilitar MYKV; 4.1 recall remoto y 4.3 CRDT están diferidos. Esta vía no transfiere payload, no añade almacenamiento durable ni prueba producción.

Fuentes del contrato integrado:
- [`mesh_udp.rs`](../crates/mycnet-daemon/src/mesh_udp.rs): wire, autorización, ACK, retry y backpressure.
- [`storage.rs`](../crates/mycnet-memory/src/storage.rs): límites, metadatos exactos y LWW.
- [`api.rs`](../crates/mycnet-daemon/src/api.rs): consulta HTTP de metadatos.
- [Especificación normativa](../openspec/specs/memory/kv-metadata-replication/spec.md).
- [Infraestructura y observaciones históricas](INFRAESTRUCTURA.md).
