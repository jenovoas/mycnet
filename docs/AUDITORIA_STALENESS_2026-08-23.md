# AUDITORÍA DE FRESCURA / STALENESS — MycNet (HEAD 6fc94f0)

**Fecha:** 2026-08-23
**Alcance:** Verificación estática (cero builds, cero git ops, cero ejecución) de los 4 crates Rust del workspace `mycnet` contra la superficie documental (91 `.md`).
**Objetivo:** Determinar qué documentación sigue vigente respecto al código real.
**Método:** Mapa de módulos real (leyendo `lib.rs`/`main.rs`/`Cargo.toml` de cada crate) + histograma de directorios + auditoría por archivo de las 8 notas vivas más centrales con citas `archivo:línea`.

> **Nota de trazabilidad.** Todo `archivo:línea` citado aquí corresponde al HEAD `6fc94f0` y se verificó leyendo el código fuente directamente. Ninguna afirmación de estaleness se basa en memoria o en headers no contrastados contra el código presente.

---

## 1. Mapa de módulos real (código, no documentación)

Un solo **crate binario** y tres **librerías**, conectados así:

```
mycnet-core  (lib, sin deps de mycnet)
   ├── s60    → SPA (aritmética point-fixed Base-60, i64-only, sin f32/f64)
   ├── adm    → ADM (red hexagonal axial, vecinos), portado de sentinel-cubepath/backend/src/mycnet.rs
   ├── mesh   → parseo de `batctl n`, vecinos/TQ(0-255)→SPA
   └── yhwh   → modulación armónica 10-5-6-5 (fases Yod/He/Vav/HeFinal)

mycnet-memory  (lib, dep = mycnet-core)
   ├── lattice          → LiquidLattice (difusión Von Neumann, canal dual Amplitud/Fase)
   ├── phase_gate       → PhaseGate (escribir 1/6 de red por tick)
   ├── sync             → SatelliteSync (pares de frontera, acoplamiento por TQ)
   ├── storage          → ResonantStore (consistencia por coherencia S60)
   ├── sentinel_bridge  → puente SHM `/me60os_lattice` con ~/Proyectos/sentinel (zero-copy FFI repr(C))
   └── holographic_sync → HolographicSeed (sync holográfico 32 bytes, proyecto LFM↔MycNet; exportado pero NO usado por daemon)

mycnet-daemon  (bin `mycnetd`, deps = core + memory)
   ├── main.rs   → loop isócrono 42 Hz (≈41.77), ticker → AppState::tick
   ├── state.rs  → AppState: ADM + liquid lattice + SentinelBridge opcional (None si sentinel-cortex no corre)
   └── api.rs    → axum: /health, /api/v1/mesh, /api/v1/adm, /api/v1/stream (WS)

mycnet-connect  (lib público, dep = ninguno del workspace)
   ├── client.rs  → MycNetClient (HTTP GET /api/v1/adm + WS /api/v1/stream)
   └── types.rs   → MeshEvent, NodeInfo, MeshSnapshot, NeighborInfo
```

**Dependencias efectivas (Cargo.toml real):**

| Crate | Deps del workspace | Deps externas |
|---|---|---|
| `mycnet-core` | — | serde, serde_json, tracing, thiserror |
| `mycnet-memory` | mycnet-core | serde, serde_json, tokio, tracing, thiserror, libc |
| `mycnet-daemon` | mycnet-core, mycnet-memory | serde, serde_json, tokio, axum(0.7, ws), tracing(-subscriber), anyhow |
| `mycnet-connect` | — | serde, serde_json, tokio, thiserror, anyhow, reqwest(0.12), tokio-tungstenite(0.24), futures-util, tracing |

**Lo que el código REALMENTE implementa:**

- **Protocolo/transporte real:** el daemon lee `batman-adv` vía `batctl n` (`mycnet-core/src/mesh.rs:86-112`), parsea vecinos con su **TQ (0–255)** y lo convierte a SPA (`tq_s60: SPA::from_tq(tq)` en `mesh.rs:33-36,91`; test `mesh.rs:169-177` confirma `tq=255→SPA::one()`). Esta es la única integración batman-adv que existe en el código.
- **TQ→S60 (routing):** `SPA::from_tq(tq: u8)` en `s60.rs:82`, con normalización a SPA `[0,1).` Es un **mapeo**, no un algoritmo de enrutamiento. No hay router propio: MycNet consume la topología que ya resuelve batman-adv.
- **Extensión Sentinel real:** puente SHM zero-copy vía `sentinel_bridge.rs` leyendo `/me60os_lattice` (contrato FFI `#[repr(C)]` 192 B/nodo vs. `SPA` 40 B). Este puente es **opcional**: si `sentinel-cortex` no corre, `AppState.sentinel = None` (`state.rs:39,66-81`) y el daemon opera **standalone**.
- **Servidor de despliegue real (código):** Ninguno. El único deployment que cita el código es `sentinel-cubepath` como **origen del port** del ADM (`adm.rs:9`), y `sentinel-cortex` como **proveedor del SHM** (`sentinel_bridge.rs:81,114,326`). No hay rastro de `fenix` ni de un despliegue propio en el código del workspace.

---

## 2. Histograma de la superficie documental (91 `.md`)

| Área | Archivos | Naturaleza |
|---|---|---|
| `docs/` (raíz de docs) | 49 | Investigación de frontera / física especulativa / hardening / notas |
| `docs/experiments/` | 20 | Experimentos EXP-001..EXP-0XX (validaciones) |
| Raíz del repo (`*.md`) | 8 | README, CLAUDE, CODEBUDDY, GEMINI, QODER, AGENTS, ENGINEERING_MANIFESTO, PROTOCOLO_YATRA |
| Copias de skills agentes (`.claude/`, `.codebuddy/`, `.gemini/`, `.kiro/`) | 12 | Skills duplicados por herramienta — **redundancia, no documentación live** |
| `docs/` otros (plan, protocolo) | 2 | MYCNET_IMPLEMENTATION_PLAN, PROTOCOLO_YATRA |

**Conteo exacto:** `docs/` = 69 (49 + 20), skills-copia = 12, raíz = 8, más 2 auxiliares (4_NODE_*). Total = 91. ✓ coincide con el claim.

**Superficie "live" (describe el mycnet real de estos 4 crates):** solo ~4 notas — `README.md`, `CLAUDE.md`, `MYCNET_IMPLEMENTATION_PLAN.md`, `PROTOCOLO_YATRA.md`. El grueso de `docs/` (45/49) es **investigación de frontera, física especulativa y hardening** que describe el ecosistema **sentinel/me-60os**, **no** el workspace mycnet (ver §3 notas S/D). Los 20 `EXP_*` son registros de experimentos históricos (validaciones), no documentación del código actual.

---

## 3. Auditoría por archivo — las 8 notas centrales

**8 elegidas:** README, CLAUDE, PROTOCOLO_YATRA, ENGINEERING_MANIFESTO, MYCNET_IMPLEMENTATION_PLAN (root/plan) + architecture_technical, Arquitectura_Hibrida, redes_micelio_hexagonal (los más citados "architectural" de docs/).

| # | Documento | Verdict | Cita de la contradicción / apoyo |
|---|---|---|---|
| 1 | `README.md` | **[DESAJUSTE]** (5 puntos #1-#5 en §4) | v1.0 claims `PAI-60`, eBPF/Ring-0, `configs/`, `cortex-bridge`, despliegue en «fenix y sentinel-cubepath» — ninguno existe en el código |
| 2 | `CLAUDE.md` | **[VIGENTE]** (stack core) | L46-35 describe correctamente los 4 crates; L27 "batman-adv (L2 Mesh)" ✓ `mesh.rs` |
| 3 | `PROTOCOLO_YATRA.md` | **[DESAJUSTE]** (#5 en §4) | L23-25 cita `src/math/s60.rs` y `src/math/spa_math.rs` — no existen; el motor es `crates/mycnet-core/src/s60.rs` |
| 4 | `ENGINEERING_MANIFESTO.md` | **[S/D — describe sentinel]** | L15 «SENTINEL-CUBEPATH... único objetivo de despliegue» es del manifiesto sentinel; L21-27 «Liquid Lattice», «LIF» refieren sentinel/me-60os, no mycnet. No es desajuste de mycnet sino de ámbito |
| 5 | `MYCNET_IMPLEMENTATION_PLAN.md` | **[VIGENTE como plan]** | L41-56 MinIO EC4+2 → Ceph, TQ en `bat0`, fq_codel/cake ✓ presente en scripts y docker-compose; plan ≠ implementación |
| 6 | `architecture_technical.md` | **[S/D — ámbito sentinel]** | L11 «orquestación resonante (Python/Time Crystal)», L30 `ai_guardian.c`, L43 `ebpf_cortex_bridge.rs` — describe me-60os/sentinel, no mycnet |
| 7 | `Arquitectura_Hibrida.md` | **[S/D — ámbito sentinel]** | L11 `sentinel_core`/`cortex_main.py`/`QuantumNode`, L6 opts — no es arquitectura de mycnet |
| 8 | `redes_micelio_hexagonal.md` | **[VIGENTE conceptual]** | L15-23 ADM/hexágonos/coherencia ✓ coincide con `adm.rs` (vecinos axiales, `AxialCoord`); es el respaldo del ADM. Se marcará "conceptual, no de molde de API" |

**Leyenda:** `[VIGENTE]` = coincide con el código; `[DESAJUSTE]` = contradice o referencia código/archivos inexistentes; `[S/D]` = sin desajuste porque **no describe este workspace** (ámbito sentinel/me-60os), no es stale de mycnet sino transversal.

### Detalle spot-audit #1 — integración batman-adv y TQ→S60

- **Claim docs:** README §Componentes «ADM... protocolo `batman-adv` (L2 Mesh)» (`README.md:21`).
- **Código:** sí, `mycnet-core/src/mesh.rs:12` `use std::process::Command`, diseccionando `batctl n` (`mesh.rs:86-112`). **VIGENTE** — la integración batman-adv es real y es la única.
- **TQ→S60:** `mesh.rs:36` `pub tq_s60: SPA`, `mesh.rs:91` `tq_s60: SPA::from_tq(tq)`, `s60.rs:82` `from_tq(tq: u8)`, test `mesh.rs:169-177` (tq=255⇒SPA::one). **VIGENTE** — es el mecanismo documentado de llevar el TQ de batman-adv a SPA. Es un **mapeo de métrica**, no un router. No existe algoritmo de enrutamiento TQ propio (depende de batman-adv).

### Detalle spot-audit #2 — despliegues de servidor (fenix / sentinel-cubepath)

- **Claim docs:** `README.md:5` «parcialmente implementado y validado en los servidores de prueba **fenix** y **sentinel-cubepath**»; `README.md:57` binario `cortex-bridge`; `README.md:67-68` «Mesh `bat0` operativo en nodos Fenix... `yhwh_driver`... en `sentinel-cubepath`».
- **Código (workspace):** no hay mención de `fenix` en ningún `.rs` (grep global: 0); `sentinel-cubepath` aparece solo como **origen del port** (`adm.rs:9`) y como proveedor del SHM vía `sentinel-cortex` (`sentinel_bridge.rs:81,114,326`, `state.rs:66-81`). El `yhwh_driver` citado en L68 **no existe como binario** — el único binario es `mycnetd` (`mycnet-daemon/Cargo.toml:7-9`). La modulación YHWH **sí existe**, pero como módulo `yhwh.rs` dentro de `mycnet-core`, orquestada por el daemon (`state.rs:100`, `api.rs:41`, `storage.rs:142`) — no es un driver/binario separado.
- **Veredicto:** la afirmación de despliegue en `fenix` **no es verificable** en el código del workspace (no hay infraestructura ni ruta de despliegue; los servidores están fuera del repo). Las afirmaciones sobre `cortex-bridge` binario y `yhwh_driver` binario son **falsas** (inexistentes). El despliegue Sentinel real es vía `sentinel-cubepath`/`sentinel-cortex` como dependencia externa, no algo que mycnet implemente.

---

## 4. Falsedades probables en README.md (correcciones quirúrgicas propuestas)

| # | README línea | Claim | Realidad (código) | Acción |
|---|---|---|---|---|
| 1 | L14 | «tabla de recíprocos **PAI-60**... divisiones en O(1)» | No existe tabla PAI-60 en el código; `s60.rs` implementa SPA con `Div` escalado, sin tabla de recíprocos | **EDITAR** (corregir a "división por enteros escalados S60", quitar PAI-60) |
| 2 | L36-39 | «Integración **eBPF (Ring 0)**... LSM `ai_guardian`» | No hay eBPF/LSM/aya en `crates/` — grep global solo da `sentinel_bridge.rs`/`state.rs` mencionando `sentinel-cortex`. El Ring-0 es de sentinel, no de mycnet | **EDITAR** (reencuadrar como "extiende sentinel vía SHM, el Ring-0 vive en sentinel") |
| 3 | L57 | Binarios «`mycnetd`, **cortex-bridge**» | Único binario: `mycnetd`. `cortex-bridge` NO existe | **EDITAR** (eliminar cortex-bridge del árbol) |
| 4 | L60 | «`PAI60_ebpf.md`» | El archivo real es `docs/PAI60_ebpf_integration.md` | **EDITAR** (nombre de archivo) |
| 5 | L61 | «`configs/` — units de systemd y dashboards Grafana» | El directorio es `config/` (singular) y contiene solo `prometheus.yml`; no hay units systemd ni dashboards | **EDITAR** (renombrar a `config/` y ajustar descripción) |
| — | L5, L67 | «fenix» como servidor de validación | No verificable en el código; `yhwh_driver` no existe como binario. El despliegue real es sentinel-cubepath/cortex como externo | **EDITAR (matiz)** — reencuadrar como "integración validada con sentinel-cubepath via SHM"; quitar mención no-verificable de `yhwh_driver` como binario (L68) |

**Nota de conservadurismo (regla AGENTS §3.1/§3.3):** solo edito estas 5 falsedades **probables** (archivos/binarios/directorios que grep + `Cargo.toml` demuestran inexistentes). No toco las afirmaciones físicas/especulativas de los Fundamentos (L11-13), que son tesis del proyecto, no hechos de código. La referencia a `fenix` como "servidor de prueba validado" se conserva **si** es una declaración operacional externa al repo, pero la menciono en pendientes para que el autor la verifique, sin eliminarla de forma irreversible salvo que el criterio sea estrictamente código.

---

## 5. Resumen de hallazgos y recomendaciones

**Lo que está VIGENTE:**
- Núcleo SPA / S60 (`s60.rs`) — motor real, i64-only, sin float.
- ADM hexagonal (`adm.rs`) con coordenadas axiales y vecinos.
- Integración batman-adv real vía `batctl` + mapeo TQ→S60 (`mesh.rs`).
- Modulación YHWH 10-5-6-5 (`yhwh.rs`) integrada en daemon + storage.
- Puente Sentinel SHM zero-copy (`sentinel_bridge.rs`) — opcional, standalone-aware.
- MinIO/Prometheus/Ceph — presentes en `docker-compose.yml` y `scripts/` ✓ plan.

**Desajustes / riesgo de documentación inflada:**
- README describe un ámbito (eBPF Ring-0, PAI-60, cortex-bridge, configs/, fenix/`yhwh_driver` binario) que **el workspace mycnet no implementa** — esos conceptos viven en el ecosistema *sentinel/me-60os*, no aquí. Riesgo: un lector creerá que mycnet trae eBPF y/binarios que no existen.
- Notas "architectural" de `docs/` (`architecture_technical.md`, `Arquitectura_Hibrida.md`, `ENGINEERING_MANIFESTO.md`) son **ámbito sentinel** — no deben leerse como arquitectura de mycnet. Útil: etiquetar frontmatter `tags: [sentinel, frontier]` para separar (dejar en pendientes; no es una falsedad a corregir aquí).

**Dejar en `Pendientes` (fuera de alcance de esta auditoría):**
- Verificar operacionalmente si `fenix` y `sentinel-cubepath` son servidores activos (fuera del repo; requiere acceso SSH, prohibido aquí).
- Decidir si `docs/*` de física especulativa deben quedar en mycnet o migrar a un vault sentinel.
- Re-tagguear las 12 copias de skills agentes (redundancia doc) si se desea consolidar.

**Conteo final de falsedades corregibles en README:** 5 con certeza (`cortex-bridge`, `configs/`, `PAI60_ebpf.md`, `PAI-60`, eBPF en mycnet), + 1 de matiz (`yhwh_driver` como binario independiente). Todas verificadas por ausencia de archivo/binario o por grep.