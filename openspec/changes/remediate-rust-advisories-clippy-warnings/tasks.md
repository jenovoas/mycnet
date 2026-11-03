# Tasks

## 1. Actualizar dependencias vulnerables

- [x] 1.1 Actualizar de forma dirigida `anyhow`, `h2` y `tokio` en el lockfile; verificar con `cargo tree -i anyhow`, `cargo tree -i h2`, `cargo tree -i tokio` y `cargo deny check advisories` que se cumplen las versiones parcheadas y desaparecieron RUSTSEC-2026-0190, RUSTSEC-2026-0258 y RUSTSEC-2026-0386.
- [x] 1.2 Ejecutar `cargo check --workspace --all-targets` y `cargo test --workspace --all-targets`; confirmar compilación y suite completa tras actualizar crates.

## 2. Corregir avisos Clippy

- [x] 2.1 Mantener la firma pública de `HolographicSeedBinary::new` y aplicar `#[expect(clippy::too_many_arguments, reason = "...")]` solo a ese método; verificar el crate con `cargo clippy -p mycnet-core --all-targets -- -D warnings` y sus pruebas.
- [x] 2.2 Sustituir las dos comprobaciones manuales de múltiplos en `crates/mycnet-memory/src/storage.rs` por la forma aceptada por el toolchain fijado; verificar con `cargo clippy -p mycnet-memory --all-targets -- -D warnings` y pruebas.
- [x] 2.3 Simplificar `map_or` en `crates/mycnet-daemon/src/mesh_udp.rs` conservando el tratamiento de observaciones nuevas; verificar los tests de `mesh_udp` y Clippy del crate con `-D warnings`.
- [x] 2.4 Eliminar el campo `PendingPacket.sequence_id` que solo se usa indirectamente en pruebas o reestructurarlo para que el test valide el identificador realmente serializado; verificar que pasan `full_peer_queue_stalls_polling_and_retains_head_until_success` y `rejected_window_keeps_seed_pending` y que no queda warning `dead_code`.

## 3. Verificación de integración

- [x] 3.1 Ejecutar `cargo check --workspace --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check advisories` y `rustfmt --edition 2021 --check` sobre los tres archivos modificados; registrar que `cargo fmt --all -- --check` detecta formato preexistente en archivos ajenos y comprobar con las pruebas que YATRA, protocolo y wire layout permanecen intactos.
- [x] 3.2 Revisar `git diff` y `git status --short`; confirmar que los cambios de código y lockfile pertenecen a esta remediación, distinguir los archivos `graphify-out` generados en la consulta previa y conservados, y verificar que no se cambiaron ajustes de GitHub ni se añadió una allowlist de licencias sin aprobación.

## Workflow follow-up

- Archivar el cambio OpenSpec después de revisar y aceptar la implementación.
- Habilitar Dependabot Security Updates solo mediante una solicitud separada y autorización explícita del usuario.
- Definir una política de licencias aprobada antes de configurar `cargo-deny` para licencias; `cargo deny check licenses` permanece como bloqueo separado mientras no exista esa decisión.
