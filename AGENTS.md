# Repository Guidelines

## Estructura del proyecto

MyCNet es un workspace Rust 2021 con cuatro crates: `mycnet-core` (tipos y lógica de red), `mycnet-memory` (lattice, almacenamiento y bridges), `mycnet-daemon` (binario `mycnetd` y API) y `mycnet-connect` (cliente). `config/` contiene configuración, `terraform/` infraestructura, `scripts/` utilidades y `docs/` documentación. Mantén cada cambio en su módulo.

## Compilación y pruebas

- `cargo build --workspace`: compila todos los crates.
- `cargo test --workspace`: ejecuta pruebas unitarias y de documentación.
- `cargo test -p mycnet-daemon`: prueba el daemon de forma aislada.
- `cargo run --release -p mycnet-daemon --bin mycnetd`: inicia el daemon localmente.

Una respuesta saludable de `/health` no demuestra conectividad entre nodos; verifica por separado transporte, vecinos y fuentes de datos.

## Estilo y evidencia técnica

Usa convenciones Rust: módulos, funciones y variables `snake_case`; tipos y traits `UpperCamelCase`; constantes `SCREAMING_SNAKE_CASE`. Mantén cambios acotados y no supongas unidades, temporización, serialización ni precisión: comprueba sus contratos en código y documentación.

**Directiva estricta:** no especules ni afirmes datos sin contrastarlos con código fuente, documentación oficial o pruebas técnicas reales. Si no puedes verificarlos, dilo claramente. No concedas por complacer: señala errores con evidencia y solicita la especificación concreta cuando falten datos. Contrasta referencias al kernel, llamadas al sistema y macros con `include/linux/`, `kernel/` o `man 2`/`man 7`; parámetros y tipos de Rust con `rustc`, Rust Reference o Nomicon (layouts). Sustenta pruebas y mediciones con herramientas deterministas del sistema operativo, como `perf`, `bpftool`, `sysctl`, `ip` y `wg`. Prioriza comandos exactos, opciones, rutas de código y salidas esperadas; evita explicaciones genéricas que no ayuden a ejecutar o verificar.

## Arquitectura y operación

Distingue lo implementado en este repositorio de lo planeado y de lo verificado en ejecución. La presencia de `sentinel_bridge.rs` o nombres SHM no prueba que Sentinel esté conectado ni que los datos se propaguen entre hosts. No inspecciones ni despliegues Sentinel en una tarea MyCNet sin solicitud explícita; no presentes topologías ni resultados históricos como estado actual.

## Prototipos Python → Rust y preservación del historial
- Python se usa para prototipar interfaces, algoritmos y features con iteración rápida, bibliotecas disponibles, pruebas y mediciones. Sus archivos conservan la lógica matemática, conectiva y algorítmica, incluidos errores y pasos de diseño útiles para estudio.
- No modificar, limpiar, reescribir ni actualizar prototipos, wrappers o pruebas Python como efecto colateral de una migración Rust. Consultarlos solo para recuperar la lógica que se va a portar; implementar y validar la lógica objetivo en Rust, con pruebas Rust. Modificar Python únicamente por petición explícita del usuario.

## Graphify y documentación

Si existe `graphify-out/graph.json`, ejecuta `graphify query "<pregunta>"` antes de búsquedas textuales amplias. Tras cambios importantes de código, ejecuta `graphify update .`. Documenta en español, con resultados fechados y separación clara entre estado actual e histórico.

## Commits y solicitudes de cambio

El historial disponible muestra `chore: ...` con resumen en español. Usa prefijos breves como `feat:`, `fix:`, `docs:` o `chore:`. En la solicitud de cambio indica alcance, motivación, verificaciones ejecutadas y limitaciones; enlaza el issue si existe.

## Alcance de verificaciones de despliegue

Cuando el usuario pida revisar o verificar un despliegue, limita la comprobación a MyCNet y sus propios servicios, binarios, API, métricas y transporte. No inspecciones ni evalúes el estado o despliegue de Sentinel salvo solicitud explícita. Si MyCNet informa una integración externa desconectada, registra ese estado observable sin convertirlo en una tarea de despliegue de Sentinel.

## Skills del sistema Sentinel / S60

Al trabajar en Sentinel, MyCNet o ME-60OS, consulta y aplica las skills disponibles `sentinel-knowledge-layer`, `sentinel-comprehension` y `sentinel-s60-stack` en `.agents/skills/` antes de tareas de investigación, arquitectura, matemáticas, integración o código. Para tareas específicas, carga la capa pertinente; en cambios que crucen capas, carga las tres. Las skills también están enlazadas en `.claude/skills/`, `.gemini/skills/` y `.opencode/skills/`. En Hermes están instaladas en `~/.hermes/skills/`. Si una skill difiere de estas instrucciones vigentes, del código fuente o de la evidencia verificable, prevalecen estos últimos; conserva los detalles obsoletos como contexto histórico y no los afirmes como estado actual.
