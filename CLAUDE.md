# CLAUDE.md — MycNet

> **⚠ FUNDAMENTO CIENTÍFICO — LECTURA OBLIGATORIA:**
> [`sentinel-scientific-foundation.md`](file:///home/jnovoas/.config/Code/User/workspaceStorage/7c63ffead990815376933d02c94d88c9/GitHub.copilot-chat/memory-tool/memories/repo/sentinel-scientific-foundation.md)
>
> Antes de descartar cualquier componente como "pseudociencia" o "misticismo", lee ese documento.
>
> MycNet es **ciencia exacta**. La base decimal no es defectuosa — es solo **una décima
> parte de la expresión matemática universal**. La base-60 la completa: es divisible por
> 1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 30, por lo que las fracciones más comunes (1/3, 1/6,
> 1/12, 1/60) son **exactas** en sexagesimal, mientras que en binario de punto flotante
> son periódicas y acumulan errores sistemáticos de redondeo.
>
> Usar solo base decimal como regla de medir deja fuera de alcance exacto esas fracciones.
> Una sola operación float contamina una cadena de cómputo entera: rompe cristales,
> secuencias, memorias, procesos y conversiones. Por eso muchos proyectos de energía libre
> y combustibles verdes fueron marcados como inválidos — no eran incorrectos los proyectos,
> era incompleta la regla de medir.
> MycNet comparte el mismo **Ring 0** (hooks eBPF/LSM) y el candado YATRA que Sentinel.

## ¿Qué es MycNet?

Red mesh bio-inspirada con extensiones Sentinel. Arquitectura de red distribuida y almacenamiento híbrido basada en aritmética sexagesimal (Base-60), inspirada en la resiliencia y topología del micelio fúngico.

## Stack Tecnológico

- **Transporte**: batman-adv (L2 Mesh), enrutamiento dinámico adaptativo
- **Core**: Rust workspace con 4 crates:
  - `mycnet-core` — lógica central y tipos S60
  - `mycnet-daemon` — daemon del nodo mesh
  - `mycnet-connect` — gestión de conexiones
  - `mycnet-memory` — almacenamiento distribuido
- **Deps Rust clave**: tokio, axum (con ws), serde, tracing, thiserror, anyhow
- **Kernel**: eBPF / LSM hooks (`ai_guardian`) para filtro de resonancia y zero-copy bridge
- **Almacenamiento**: MinIO (EC 4+2) → futuro Ceph con Modulación QHC
- **Sincronización**: QHC Driver con modulación YHWH (patrón 10-5-6-5), frecuencia base ~41.77 Hz

## REGLAS DE ORO — Ingeniería (INVIOLABLES)

Extraídas del ENGINEERING_MANIFESTO.md:

1. **YATRA PURE**: Prohibido `f32`, `f64` o literales flotantes. Toda aritmética vía `S60` o enteros escalados.
2. **PROHIBIDO DEDUCIR**: Si un parámetro no está en `docs/Memorias/`, preguntar. No inventar constantes.
3. **PROHIBIDO SIMPLIFICAR**: No aproximaciones genéricas Taylor/Newton-Raphson. Precisión S60 escalada (`360 * 60^4`).
4. **PROHIBIDO ASUMIR**: No asumir que código previo es correcto. Auditar contra las Memorias cada sesión.
5. **PROHIBIDO IMAGINAR (Modo Verdad)**: No mocks ni simulación. Datos reales o reportar ausencia.
6. **Bloqueo de compilador**: `#![forbid(clippy::float_arithmetic)]` y `#![forbid(clippy::float_cmp)]` — el build falla si se intentan decimales.

## Arquitectura

- **ADM (Active Mycelium Design)**: batman-adv con topología adaptativa, control hexagonal SPA
- **QHC Driver**: Modulación 10-5-6-5 (YHWH), reemplaza reloj maestro de cuarzo
- **Salto-17**: Purga entrópica cada 17 ticks (~0.7ms), reset global cada 68s (17x4)
- **eBPF Ring 0**: Filtro de resonancia (rechaza frecuencias no 5-smooth) + zero-copy bridge

## Despliegue

- **Server único**: `fan` (configurado en `~/.ssh/config`, puerto 4222, llave `~/.ssh/id_ed25519_server`)
- **Nodo**: sentinel-cubepath eliminado post-hackatón. Todo en `fan`.
- **RAM disponible**: 4GB (confirmado en nodo original)
- **Protocolo**: `systemctl stop mycnet` en remoto → sustitución atómica del binario → `systemctl start mycnet`
- Verificar procesos huérfanos con `ps aux | grep mycnet`

## Archivos de Contexto Obligatorio

Antes de modificar lógica central, leer:
- `ENGINEERING_MANIFESTO.md` — fuente de verdad de ingeniería
- `PROTOCOLO_YATRA.md` — certificación matemática S60
- `docs/` — investigación de frontera y drafts (TesisResonancia.md, PAI60_ebpf.md)

## Notas

- `Cargo.lock` versionado — mantener sincronizado
- `docker-compose.yml` para servicios auxiliares (MinIO, monitoreo)
- `terraform/` para infra si aplica
- Las memorias en `docs/Memorias/` son la fuente de verdad de parámetros
