# 🕸️ MycNet: Red Mesh Bio-Inspirada & Almacenamiento Resonante S60

**MycNet** es una arquitectura de red distribuida y almacenamiento híbrido bio-inspirado que opera bajo principios de **Resonancia Armónica**, **Compresión Fractal por Semilla** y **Aritmética Sexagesimal Base-60 ($S_{60}$)**, eliminando el error de truncamiento y la entropía de los sistemas decimales tradicionales.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](./LICENSE)
[![Rust 2021+](https://img.shields.io/badge/rust-2021%2B-black.svg?logo=rust)](./crates/mycnet-core/)
[![Core S60](https://img.shields.io/badge/Core-S60%20Fixed%20Point-purple)](./crates/mycnet-core/)
[![WireGuard Mesh](https://img.shields.io/badge/Mesh-WireGuard%2010.88.0.0%2F24-emerald)](./)

---

## 🏛️ Topología de Red y Enlace (Fenix $\leftrightarrow$ Kingu)

> **Interconexión:** MycNet opera transportando paquetes y estados de coherencia sobre el túnel cifrado **WireGuard (`10.88.0.0/24`)** uniendo Fenix (`10.88.0.1`, Cuenta Azure A) y Kingu (`10.88.0.2`, Cuenta Azure B).

```
     ┌──────────────────────────────────┐    ┌──────────────────────────────────┐
     │      FENIX (Cuenta Azure A)      │    │      KINGU (Cuenta Azure B)      │
     │      Túnel VPN: 10.88.0.1/24     │    │      Túnel VPN: 10.88.0.2/24     │
     ├──────────────────────────────────┤    ├──────────────────────────────────┤
     │ • Sentinel Cortex API (:8000)    │    │ • MycNet Daemon Kingu (:7474)    │
     │ • SHM (/me60os_lattice, 67.951 N)│    │ • Sentinel Verifier Node         │
     │ • MycNet Daemon Fenix (:7474)    │    │ • Traefik v3 + Podman Apps       │
     └─────────────────┬────────────────┘    └────────────────┬─────────────────┘
                       │                                      │
                       └───────────► Túnel Cifrado ◄──────────┘
                                  WireGuard (wg0)
                                   10.88.0.0/24
```

---

## 🔬 Los 3 Niveles de Memoria y Resonancia

1. **Nivel 1: Simpatía Hexagonal:**
   - Difusión de energía: cada nodo transfiere `flow = (diff * coupling_factor) / 6` a sus 6 vecinos. Ante la falla de un nodo, los vecinos rellenan y recargan su amplitud colectivamente.
2. **Nivel 2: Bridge SHM Zero-Copy (`/me60os_lattice`):**
   - Lectura atómica de 192 bytes por oscilador isocrónico (`#[repr(C)]`) directamente desde memoria compartida POSIX, sin serializaciones intermedias.
3. **Nivel 3: Compresión Fractal por Semilla:**
   - La malla no transporta datos pesados crudos: almacena la **semilla fractal** y la regla de propagación, reconstruyendo el estado holográfico mediante iteración determinista en destino.

---

## 📦 Workspace Cargo (4 Crates)

```
mycnet/
├── crates/
│   ├── mycnet-core/       Tipos numéricos S60, vectores SPA, modulación YHWH, TQ a SPA
│   ├── mycnet-connect/    Capa de transporte UDP/Mesh, serialización de frames binarios
│   ├── mycnet-memory/     Retículo líquido hexagonal (LiquidLattice), bridge SHM con Sentinel
│   └── mycnet-daemon/     Binario de producción 'mycnetd' (:7474), estado global y API REST
├── scripts/
│   └── benchmark_mycnet_transfer.py   Suite de benchmarking de ancho de banda y latencia
└── config/                Configuraciones de monitoreo y reglas de red
```

---

## ⚡ Benchmarking y Comandos Rápidos

```bash
# Compilar todo el workspace en release (genera target/release/mycnetd)
cargo build --release

# Ejecutar suite de pruebas unitarias (19/19 tests)
cargo test

# Iniciar daemon de malla MycNet en puerto :7474
cargo run --release -p mycnet-daemon --bin mycnetd

# Ejecutar suite de benchmarking de transferencia y latencia
python3 scripts/benchmark_mycnet_transfer.py
```

---

## 📄 Licencia

Apache 2.0 con cláusula de uso soberano — Ver [LICENSE](./LICENSE).
Desarrollado por **Jaime Novoa Sepúlveda** para **MycNet / SecurePenguin**.
