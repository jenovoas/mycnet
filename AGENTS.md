# ARQUITECTURA Y REGLAS DE DESARROLLO (MYCNET / PROTOCOLO YATRA)

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
