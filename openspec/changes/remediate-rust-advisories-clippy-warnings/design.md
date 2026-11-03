# Design

## Context

Verificado en este checkout: `cargo deny check advisories` informa RUSTSEC-2026-0190 (`anyhow 1.0.102`), RUSTSEC-2026-0258 (`h2 0.4.13`) y RUSTSEC-2026-0386 (`tokio 1.50.0`). Clippy informa `too_many_arguments` en `s60.rs`, dos usos manuales de múltiplos en `storage.rs`, un `map_or` simplificable y `PendingPacket.sequence_id` sin lectura en compilación normal. `cargo test --workspace --all-targets` pasa en el estado base. No hay workflows de GitHub Actions versionados y `cargo-deny` no encuentra configuración de proyecto; su chequeo global falla por licencias no permitidas por defecto.

La herramienta Clippy disponible corresponde al toolchain fijado en `rust-toolchain.toml` (Rust 1.99.1). Deben preservarse las reglas YATRA, las invariantes de paquetes y la semántica de envío.

## Goals / Non-Goals

**Goals:**

- Actualizar el lockfile de manera mínima para resolver los tres advisories, sin cambios de versión mayor en dependencias directas.
- Eliminar los avisos Clippy reportados sin ocultarlos con `allow` ni alterar resultados matemáticos.
- Verificar de forma reproducible que el lockfile queda libre de los advisories identificados.
- Demostrar que compilación, pruebas y chequeos de seguridad pasan después del cambio.

**Non-Goals:**

- Cambiar versiones declaradas de dependencias directas si una actualización del lockfile basta.
- Cambiar el protocolo MycNet, formatos wire, algoritmos S60, constants, despliegues o datos remotos.
- Activar Dependabot, Code Scanning u otras funciones de seguridad en GitHub; esto requiere autorización aparte por ser un cambio compartido.
- Añadir silenciamientos globales de Clippy o aceptar licencias sin revisión.

## Decisions

- **Actualizar dependencias transitivas/directas en el lockfile antes que cambiar manifests:** conserva la API y el rango de versiones elegido para el workspace. Usar actualizaciones dirigidas para `anyhow`, `h2` y `tokio`; si la resolución mínima no permite eliminar un advisory, inspeccionar el grafo y ajustar solo el manifest responsable. Verificar que las versiones resueltas cumplen los límites de cada advisory.
- **Resolver cada warning en el punto de origen:** aplicar la simplificación sugerida por Clippy donde conserva semántica. Para el constructor público de ocho parámetros, conservar la firma por decisión del usuario y usar `#[expect(clippy::too_many_arguments, reason = "...)]` en ese método solamente; no cambiar la API ni silenciar otros avisos. Para `sequence_id`, conservar la comprobación de orden de cola en pruebas mediante los bytes/identificador realmente transportados, evitando un campo de producción no leído.
- **No alterar una allowlist de licencias en esta remediación:** el chequeo de licencias actual falla porque no existe política de proyecto. Registrar ese bloqueo y solicitar aprobación antes de permitir identificadores; no deducir compatibilidad de la licencia del propio proyecto.
- **Mantener GitHub fuera de la remediación local:** Dependabot Security Updates consta como deshabilitado. El lockfile y los análisis locales sí pueden prepararse; habilitar alertas o actualizaciones automáticas queda fuera del alcance autorizado.

## Risks / Trade-offs

- [La actualización de dependencias cambia código compilado incluso si conserva APIs] → actualizar de forma dirigida y ejecutar toda la suite del workspace, incluyendo chequeo y Clippy.
- [Una dependencia transitiva podría fijar un rango que impida el parche] → identificar el crate padre con `cargo tree -i <crate>` y hacer el menor cambio compatible; no forzar versiones incompatibles.
- [La sugerencia `is_multiple_of` podría elevar requisitos si se cambia el toolchain] → el toolchain del repositorio es 1.99.1; conservarlo y validar con ese toolchain fijado.
- [El chequeo completo `cargo deny check` seguirá fallando por licencias no configuradas] → limitar la aceptación de esta remediación a `cargo deny check advisories`; registrar el fallo separado y no añadir permisos sin aprobación.
- [Las alertas del portal GitHub no son consultables mientras estén desactivadas] → reportar evidencia de `cargo-deny` como análisis local, sin afirmar que representa una lista de alertas de Dependabot.

## Migration Plan

1. Confirmar el estado limpio/alcance de archivos y actualizar dependencias de manera dirigida.
2. Resolver los cinco warnings sin cambios funcionales; mantener sin cambios la política de licencias.
3. Ejecutar `cargo check --workspace --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings` y `cargo deny check advisories`. Registrar separadamente que `cargo deny check licenses` sigue pendiente de una política aprobada.
4. Revisar `git diff` y `git status`; no desplegar ni cambiar ajustes remotos.

Rollback: revertir los cambios locales de código, configuración y lockfile del cambio si fallan las pruebas o aparece incompatibilidad; no modificar ni restaurar cambios ajenos.

## Open Questions

Ninguna para el alcance local descrito. La configuración de licencias de `cargo-deny` y la activación de Dependabot quedan expresamente fuera de este cambio y requieren decisiones/autorizaciones separadas.
