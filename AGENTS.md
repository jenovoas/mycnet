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

## Conocimiento del Código y Navegación (graphify)

Este proyecto cuenta con soporte de grafo de conocimiento local mediante **graphify**.

### Reglas de consulta e interacción:
- Si existe `graphify-out/graph.json`, ejecuta `graphify query "<pregunta>"` antes de realizar búsquedas textuales masivas (`grep`/`find`).
- Utiliza `graphify path "<A>" "<B>"` para rastrear dependencias o relaciones entre dos módulos o componentes.
- Utiliza `graphify explain "<concepto>"` para explicaciones enfocadas de nodos y su vecindario arquitectónico.
- Si existe `graphify-out/wiki/index.md` o `graphify-out/GRAPH_REPORT.md`, consúltalos para navegación arquitectónica global.
- Tras realizar cambios importantes en el código, ejecuta `graphify update .` para mantener el grafo sincronizado (análisis AST local sin consumo de tokens de API).
- Para construir o reconstruir el grafo completo, utiliza el comando `/graphify .`.

## Alcance de las verificaciones de despliegue
- Cuando el usuario pida revisar o verificar el despliegue, limitar la comprobación a MycNet y sus propios servicios, binarios, API, métricas y transporte.
- No inspeccionar ni evaluar el estado o despliegue de Sentinel salvo que el usuario lo solicite explícitamente. La disponibilidad de una integración externa puede anotarse solo como estado observable desde MycNet, sin convertirla en una tarea de despliegue de Sentinel.
