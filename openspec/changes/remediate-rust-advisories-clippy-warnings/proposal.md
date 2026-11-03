# Proposal

## Why

La auditoría local del workspace detectó tres advisories Rust en dependencias bloqueadas, cinco warnings reproducibles de Clippy y una comprobación de licencias que no puede evaluar el proyecto por falta de configuración. Corregir los advisories reduce riesgos conocidos sin cambiar el comportamiento del protocolo; limpiar los warnings deja comprobaciones locales más útiles y confiables.

## What Changes

- Actualizar en `Cargo.lock` `anyhow`, `h2` y `tokio` a versiones que resuelvan respectivamente RUSTSEC-2026-0190, RUSTSEC-2026-0258 y RUSTSEC-2026-0386, conservando las restricciones declaradas en los manifests y verificando el grafo completo.
- Corregir los avisos de Clippy confirmados en `mycnet-core`, `mycnet-memory` y `mycnet-daemon` sin cambiar cálculos, formato de paquetes ni semántica de envío.
- Ejecutar y registrar chequeos reproducibles para compilación, pruebas, Clippy y advisories. Investigar el fallo actual de licencias sin añadir una allowlist sin aprobación explícita.
- No habilitar ni modificar ajustes de seguridad del repositorio en GitHub; Dependabot está desactivado y su activación requiere autorización separada.

## Capabilities

### New Capabilities

Ninguna.

### Modified Capabilities

Ninguna. Este cambio es mantenimiento de dependencias y tooling y no modifica requisitos ni comportamiento funcional; por eso `.openspec.yaml` declara `skip_specs: true`.

## Impact

Afecta `Cargo.lock`, código Rust en `crates/mycnet-core`, `crates/mycnet-memory` y `crates/mycnet-daemon`, y configuración de `cargo-deny`. No cambia APIs públicas intencionalmente, el protocolo de red, la aritmética S60/YATRA ni la configuración remota de GitHub. La consulta de Dependabot por API no está disponible porque las alertas están deshabilitadas; los advisories reportados proceden del análisis local de `cargo-deny` contra Cargo.lock.
