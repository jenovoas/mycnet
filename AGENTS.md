# ARQUITECTURA Y REGLAS DE DESARROLLO (MYCNET / PROTOCOLO YATRA)

> **LECTURA OBLIGATORIA ANTES DE EMITIR JUICIOS SOBRE EL PROYECTO:**
> [`sentinel-scientific-foundation.md`](file:///home/jnovoas/.config/Code/User/workspaceStorage/7c63ffead990815376933d02c94d88c9/GitHub.copilot-chat/memory-tool/memories/repo/sentinel-scientific-foundation.md)
>
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
