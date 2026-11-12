# Candado YATRA para MyCNet — 2026-11-12

## Regla

Los crates Rust de MyCNet admiten solo aritmética entera escalada SPA/S60 o SPAMath. La presencia de tipos `f32`/`f64` o literales de punto flotante en fuentes Rust bloquea la compilación Cargo.

## Implementación

- `scripts/yatra_build_guard.rs` analiza con `syn` todas las fuentes Rust de los miembros declarados por el espacio de trabajo, incluidas pruebas, binarios, ejemplos, `build.rs` y tokens dentro de macros.
- Los cuatro crates (`mycnet-core`, `mycnet-memory`, `mycnet-daemon` y `mycnet-connect`) llaman al guard desde su `build.rs`; compilar cualquier paquete miembro ejecuta la inspección del workspace.
- La inspección falla cerrada si no puede enumerar, leer o analizar una fuente.

## Evidencia local

- `cargo check --workspace --all-targets` — aprobado; no encontró floats existentes en MyCNet.
- Commit publicado en `main`: `3053fb4` (`fix(yatra): bloquear punto flotante en compilaciones`). Verificaciones locales: `cargo check --workspace --all-targets`, build release y `cargo test --workspace --locked --offline` aprobados (48 pruebas y 1 prueba de documentación). No hay ejecuciones de workflow de MyCNet en GitHub.
- No se desplegó ni reinició `mycnetd`: el cambio solo afecta compilaciones.

## Alcance

La regla cubre las compilaciones Cargo de los crates MyCNet y sus fuentes. No modifica el binario global `rustc` ni inspecciona código que una macro de dependencia externa genere internamente.
