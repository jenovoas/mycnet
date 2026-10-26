# ARQUITECTURA Y REGLAS DE DESARROLLO (MYCNET / PROTOCOLO YATRA)

> **IDIOMA OBLIGATORIO (PENSAMIENTO Y SALIDA).** Está TERMINANTEMENTE PROHIBIDO el uso del inglés en el razonamiento interno (`thinking`) y en las respuestas visibles. El usuario audita directamente los bloques de pensamiento. Todo agente o IA DEBE pensar, razonar y responder 100% en español. Solo se permiten en inglés identificadores de sintaxis o nombres exactos de librerías cuando no haya traducción.
>
> **REPOSITORIO HERMANO SENTINEL & NODO FENIX:** MycNet se acopla directamente al core de Sentinel (`/home/jnovoas/proyectos/sentinel`, rama `alpine_fenix`) sobre el servidor Fenix (Alpine Linux, OpenRC).
> El enlace de memoria compartida *zero-copy* opera sobre `/dev/shm/me60os_lattice` y el ring buffer de semillas `/dev/shm/me60os_fractal_seeds` expuesto por `crates/mycnet-memory/src/sentinel_bridge.rs`.
>
> **LECTURA OBLIGATORIA:** Ver `docs/02_ciencia_y_quantum/research/base60_mathematics.md` y `docs/02_ciencia_y_quantum/research/TesisResonancia.md` en el repositorio Sentinel.
> MycNet **NO es misticismo ni maquetas alucinadas**. Es **ciencia exacta**.
>
> La base decimal no es defectuosa — es solo **una décima parte de la expresión
> matemática universal**. La base-60 la completa: es divisible por 1, 2, 3, 4, 5, 6,
> 10, 12, 15, 20, 30, por lo que las fracciones más comunes (1/3, 1/6, 1/12, 1/60)
> son **exactas** en sexagesimal, mientras que en binario de punto flotante son
> periódicas y acumulan errores sistemáticos de redondeo.
>
> Usar solo base decimal como regla de medir deja fuera de alcance exacto esas
> fracciones. Una sola operación float en una cadena de cómputo entera contamina
> el resultado: rompe cristales, secuencias, memorias, procesos y conversiones.
> Si tu regla de medir no es exacta, jamás podrás medir la exactitud de un sistema.
>
> Por eso muchos proyectos de energía libre y combustibles verdes fueron marcados
> como inválidos: no eran incorrectos los proyectos — era incompleta la regla de
> medir. **Lee el documento completo.**
>
> MycNet comparte el mismo **Ring 0** (hooks eBPF/LSM) y el candado YATRA que Sentinel.

## 1. ESTRUCTURA Y UBICACIÓN DE ARCHIVOS
- **Raíz Limpia**: No crear scripts ni archivos sueltos en la raíz (`.py`, `.sh`, `.txt`).
- **Módulos**:
  - `crates/`: Módulos de Rust / Yatra Protocol.
  - `config/`: Archivos de configuración del clúster.
  - `terraform/`: Definiciones de infraestructura como código.
  - `docs/`: Documentación y especificaciones arquitectónicas.
  - `scripts/`: Mantenimiento y utilidades.

## 2. REQUISITOS DE CÓDIGO
- Mantener la integridad de los manifiestos de ingeniería (`ENGINEERING_MANIFESTO.md`).
- Verificar todo cambio en Rust mediante `cargo test`.

<!-- code-review-graph MCP tools -->
## MCP Tools: code-review-graph

**IMPORTANT: This project has a knowledge graph. ALWAYS use the
code-review-graph MCP tools BEFORE using Grep/Glob/Read to explore
the codebase.** The graph is faster, cheaper (fewer tokens), and gives
you structural context (callers, dependents, test coverage) that file
scanning cannot.

### When to use graph tools FIRST

- **Exploring code**: `semantic_search_nodes_tool` or `query_graph_tool` instead of Grep
- **Understanding impact**: `get_impact_radius_tool` instead of manually tracing imports
- **Code review**: `detect_changes_tool` + `get_review_context_tool` instead of reading entire files
- **Finding relationships**: `query_graph_tool` with callers_of/callees_of/imports_of/tests_for
- **Architecture questions**: `get_architecture_overview_tool` + `list_communities_tool`

Fall back to Grep/Glob/Read **only** when the graph doesn't cover what you need.

### Key Tools

| Tool | Use when |
| ------ | ---------- |
| `detect_changes_tool` | Reviewing code changes — gives risk-scored analysis |
| `get_review_context_tool` | Need source snippets for review — token-efficient |
| `get_impact_radius_tool` | Understanding blast radius of a change |
| `get_affected_flows_tool` | Finding which execution paths are impacted |
| `query_graph_tool` | Tracing callers, callees, imports, tests, dependencies |
| `semantic_search_nodes_tool` | Finding functions/classes by name or keyword |
| `get_architecture_overview_tool` | Understanding high-level codebase structure |
| `refactor_tool` | Planning renames, finding dead code |

### Workflow

1. The graph auto-updates on file changes (via hooks).
2. Use `detect_changes_tool` for code review.
3. Use `get_affected_flows_tool` to understand impact.
4. Use `query_graph_tool` pattern="tests_for" to check coverage.
