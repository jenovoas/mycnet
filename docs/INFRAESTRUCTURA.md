# Infraestructura Actual — MycNet

> **Última actualización:** 2026-08-24
> Este documento es la **fuente de verdad** de la infraestructura viva del proyecto.
> Cualquier referencia a otros servidores en `docs/` es histórica (ver § Infraestructura retirada).

---

## 🖥 Nodo único: `fenix` (Azure)

| Propiedad | Valor |
|---|---|
| Hostname | `fenix` |
| Proveedor | Microsoft Azure (VM) |
| Región | **Brazil South** (`brazilsouth`) |
| Resource Group | `feni_grupo` |
| Tamaño | `Standard_D4as_v6` — 4 vCPU / 15 GiB RAM |
| OS | Ubuntu 24.04.4 LTS |
| Kernel | `6.17.0-1022-azure` |
| Disco | 123 GB (~60 GB usados) |
| IP pública | `20.226.112.222` |
| Red interna | `eth0`: `10.0.0.4/24` |
| VPN | WireGuard `wg0`: `10.88.0.1/24` |
| En línea desde | 2026-08-23 |

---

## 🛡 Ring 0 (eBPF/LSM) — ACTIVO

Programas pineados en `/sys/fs/bpf` en este nodo:

- `ai_guardian` / `lsm_ai_guardian` — módulo LSM principal
- `guardian_alpha_lsm`, `guardian_cognitive`
- `cortex_events` — ring buffer de eventos (zero-copy bridge)
- `float_detector` — detección de aritmética flotante (vigilancia del paradigma S60)
- `gamma_heartbeat`
- `sentinel`

El candado YATRA y el Ring 0 compartido con Sentinel están **operativos** en este nodo.

---

## 🗑 Infraestructura retirada

| Recurso | Estado |
|---|---|
| `fenix` (GCloud) | **Eliminado.** El fenix actual es una VM nueva en Azure; no comparte nada con el anterior salvo el hostname. |
| `sentinel-cubepath` / `vps23309.cubepath.net` | **Eliminado** post-hackatón CubePath 2026. |
| Terraform CubePath (`terraform/mycnet_nodes.tf`) | **LEGACY** — conservado solo como registro histórico del hackatón. No ejecutar. |

---

## 🌐 Topología Mesh Actual (WireGuard + VXLAN + batman-adv)

El script `scripts/mesh_setup.sh` sincronizado desde el entorno productivo establece una topología de **2 nodos** punto a punto:

| Nodo | Nombre | Rol | IP WireGuard | IP Mesh (`bat0`) | Puerto WG |
|---|---|---|---|---|---|
| 1 | `laptop` | Cliente / Nodo Local | `10.88.0.2/24` | `10.10.0.11/24` | `51820` |
| 2 | `fenix` (o `fan`) | Servidor Azure (este nodo) | `10.88.0.1/24` | `10.10.0.12/24` | `51820` |

- **Encapsulación**: WireGuard (`wg0`) tuneliza paquetes UDP, sobre el cual corre **VXLAN** (ID 42) y sobre este opera **batman-adv** (`bat0`).
- **Servicio Systemd**: `systemd/mycnet-interceptor.service` captura telemetría de vecinos mesh y la envía a Redis con hardening de seguridad Linux.

---

## 🔮 Diferenciador Arquitectónico: Compresión Fractal (Solo en MycNet)

A diferencia del proyecto Sentinel (que solo contiene investigación teórica preliminar), **MycNet es el único repositorio que implementa en código de producción la Compresión Fractal**:

- **`HolographicSyncEngine`** (`crates/mycnet-memory/src/holographic_sync.rs`): Sincronización holográfica con **cero payload de red**.
- **Semillas Fractales (`HolographicSeed`)**: Paquetes de solo 32 bytes derivados de tokens LFM y sectores de fase S60.
- **Expansión y Reconstrucción**: El nodo remoto regenera el estado completo en su `Liquid Lattice` (`ResonantStore`) sin transferir datos masivos por el enlace mesh.

---

## ⏳ Trabajo diferido / pendiente

- **Terraform (IaC)**: nunca se implementó completamente — quedó diferido. El único `.tf` existente es el LEGACY de CubePath (ver arriba). El `fenix` actual de Azure fue aprovisionado manualmente, sin IaC.
- **Pods / stack de contenedores**: definidos pero **no desplegados** — también diferido. `docker-compose.yml` describe MinIO (EC 4+2), Prometheus y Grafana, pero en `fenix` no hay Docker activo ni servicios corriendo.
- **batman-adv (`bat0`)**: no instalado. La mesh L2 no está desplegada; el nodo opera standalone.
- **MinIO / Ceph**: almacenamiento distribuido pendiente (fase de validación EC 4+2).
- **Topología multi-nodo hexagonal**: el diseño (1 primario + 6 hexagonales) sigue vigente como arquitectura objetivo, pero no hay nodos adicionales aprovisionados.
- **`yhwh_driver`**: validación de deriva pendiente de reanudar en el nuevo nodo.
