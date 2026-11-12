# memory/kv-metadata-replication Specification

## Purpose

Definir la conducta observable del daemon MyCNet al replicar metadatos KV exactos por MYKV v1, con confirmación de todos los peers configurados, resolución determinista de conflictos y límites volátiles explícitos. No especifica transferencia del payload ni acredita despliegue. Fuente integrada: `e8cb1ad`; guía y evidencia histórica en [MYKV_V1.md](../../../../docs/MYKV_V1.md).

## Requirements

### Requirement: Exact MYKV v1 datagrams

El daemon SHALL aceptar MYKV v1 con tamaño exacto `80 + key_len`: magic ASCII MYKV en 0–3, tipo 1/2/3/4 en 4, versión 1 en 5, longitud u16 LE en 6–7, timestamp u64 LE en 8–15, semilla de 32 bytes en 16–47, hash BLAKE3 de 32 bytes en 48–79 y clave UTF-8 de 1–512 bytes desde 80. SHALL rechazar formatos, CRC o coordenadas de semilla inválidos.

#### Scenario: Malformed frame rejection
- **WHEN** llega un datagrama con versión, tipo, tamaño, UTF-8, CRC o coordenadas inválidos
- **THEN** el daemon no aplica metadatos ni emite ACK para ese datagrama.

#### Scenario: Full identity preservation
- **WHEN** el daemon transforma Publish en Replicate, Ack o Committed
- **THEN** conserva exactamente clave, timestamp, semilla y hash de esa identidad.

### Requirement: Restricted keys and origins

El daemon SHALL admitir solo claves con prefijo `crystal-` o `portal-metrics-`, dentro de 512 bytes UTF-8. Publish SHALL requerir IP loopback; Replicate y Ack SHALL requerir coincidencia exacta de IP y puerto con un peer configurado. Committed SHALL ser solo salida del daemon. Estas comprobaciones no constituyen autenticación criptográfica.

#### Scenario: Unauthorized publication or replica
- **WHEN** llega Publish desde IP no loopback, Replicate/Ack desde un origen no configurado o una clave no permitida
- **THEN** no se aplica ni confirma esa entrada.

#### Scenario: Incoming committed frame
- **WHEN** el daemon recibe Committed incluso desde un peer configurado
- **THEN** no lo usa para modificar metadatos ni confirmar pendientes.

### Requirement: Deterministic exact metadata LWW

El store SHALL conservar por clave el máximo lexicográfico `(timestamp_ns, payload_hash, seed_bytes)`, manteniendo semilla y hash exactos y separados del índice legacy. Duplicados y versiones superadas SHALL no reemplazar la ganadora ni reinyectar el lattice. El timestamp original SHALL conservarse en reintentos; no representa un reloj causal global.

#### Scenario: Equal timestamps and reordered versions
- **WHEN** se reciben para una clave versiones con igual timestamp en distinto orden
- **THEN** gana el mayor hash lexicográfico y, si también empata, la mayor semilla lexicográfica.

#### Scenario: Duplicate after dissipation
- **WHEN** se recibe de nuevo una versión ya aplicada aunque el lattice haya disipado
- **THEN** los metadatos ganadores se conservan sin nueva inyección.

### Requirement: ACK after accepted application

El daemon SHALL emitir Ack de la identidad recibida solo después de aceptación LWW: aplicada, duplicada o superada. Un error de capacidad o aplicación SHALL no emitir Ack. Un Ack SHALL satisfacer únicamente el peer pendiente y la identidad completa coincidente, no otra publicación de la misma clave.

#### Scenario: Superseded replica
- **WHEN** un peer autorizado replica una versión válida inferior a la ganadora local
- **THEN** recibe Ack de la versión recibida sin reemplazar ni reinyectar la ganadora.

#### Scenario: Wrong identity acknowledgement
- **WHEN** un peer envía Ack con hash, semilla, timestamp o clave distintos del pendiente
- **THEN** ese Ack no completa la publicación pendiente.

### Requirement: All-peer delivery confirmation

Con peers configurados, el daemon SHALL aplicar Publish aceptado localmente y replicarlo a todos ellos. SHALL reintentar cada 100 ms solo hacia peers sin Ack, conservando identidad. Committed SHALL enviarse al productor únicamente tras Ack de todos los peers configurados; sin peers SHALL no aplicar ni confirmar Publish. No se admite quorum parcial.

#### Scenario: Missing peer
- **WHEN** un peer configurado no responde aunque los demás hayan aplicado la réplica
- **THEN** no se emite Committed y se conserva el pendiente para reintento.

#### Scenario: Lost committed response
- **WHEN** se pierde Committed y el productor vuelve a enviar la misma identidad ya completada
- **THEN** el daemon reúne nuevamente Ack de todos los peers antes de otro Committed, sin reinyectar el duplicado.

### Requirement: Bounded volatile delivery state

El daemon SHALL limitar pendientes a 256 identidades globales, con un solo destinatario local por identidad. Con la capacidad llena SHALL rechazar otra identidad sin almacenarla ni confirmarla. Un Publish idéntico pendiente SHALL actualizar solo su destinatario local. Un timeout del productor no implica ausencia de réplica ni elimina el pendiente; estado y ACK no son durables.

#### Scenario: Pending capacity reached
- **WHEN** existen 256 identidades pendientes y llega otra identidad
- **THEN** no se admite ni confirma la nueva publicación y se mantienen los pendientes existentes.

#### Scenario: Retrying with a different local socket
- **WHEN** la misma identidad pendiente llega desde otro socket loopback
- **THEN** el futuro Committed se dirige al último remitente sin añadir otro pendiente ni waiter.

### Requirement: Bounded exact metadata store

El store SHALL admitir como máximo 4096 claves MYKV sin eviction de versiones LWW. Alcanzado el límite SHALL rechazar claves nuevas sin Ack ni Committed, pero permitir duplicados y actualizaciones válidas de claves existentes. El store SHALL ser volátil, sin garantía de durabilidad ni restauración automática tras reinicio.

#### Scenario: Full store with existing key
- **WHEN** hay 4096 claves y llega una réplica válida de una clave existente
- **THEN** se resuelve LWW y se emite Ack si la aplicación es aceptada.

#### Scenario: Full store with new key
- **WHEN** hay 4096 claves y llega una réplica de una clave nueva
- **THEN** no se incorpora la clave ni se emite Ack.

### Requirement: Local metadata-only HTTP lookup

`GET /api/v1/kv-metadata?key=…` SHALL consultar solo metadatos locales: HTTP 200 con `key`, `present` y `payload_available:false`; si presente, también `timestamp_ns` u64 y `seed_bytes`/`payload_hash` como arrays de 32 bytes. Claves no permitidas SHALL devolver HTTP 400. SHALL no devolver payload ni consultar peers; el debug legacy `/api/v1/kv/:token` permanece separado.

#### Scenario: Present metadata lookup
- **WHEN** se consulta una clave permitida presente
- **THEN** se devuelve su versión exacta con `present:true` y `payload_available:false`, sin campo payload.

#### Scenario: Missing metadata lookup
- **WHEN** se consulta una clave permitida ausente
- **THEN** se devuelve HTTP 200 con `present:false` y `payload_available:false`, sin timestamp, semilla ni hash.

#### Scenario: Forbidden metadata lookup
- **WHEN** se consulta una clave fuera de prefijos permitidos o del límite de bytes
- **THEN** se devuelve HTTP 400 con `error: "clave fuera de whitelist"`.

### Requirement: Independent metadata channel

MYKV SHALL compartir el listener UDP de MyCNet sin cambiar el layout legacy fractal de 40 bytes, su ring SHM, tick ni ventanas OU. La admisión y reintentos KV SHALL ser independientes de la cola predictiva fractal. El canal SHALL transportar solo metadatos, sin ofrecer lectura del payload original, recall remoto o CRDT.

#### Scenario: Legacy traffic alongside metadata
- **WHEN** el daemon recibe tráfico fractal legacy y MYKV en el mismo listener
- **THEN** procesa MYKV mediante su entrega independiente y conserva el contrato legacy de 40 bytes.
