# MycNet: Red Mesh Bio-Inspirada con Extensiones Sentinel

**MycNet** es una arquitectura de red distribuida y almacenamiento híbrido diseñada para operar bajo los principios de **Resonancia Armónica** y **Aritmética Sexagesimal (Base-60)**. Inspirada en la resiliencia y topología del micelio fúngico, busca minimizar la entropía computacional y eliminar el error de truncamiento inherente a los sistemas decimales tradicionales.

El proyecto opera actualmente sobre un único nodo: **`fenix`** (VM Azure, Brazil South). Ver [`docs/INFRAESTRUCTURA.md`](docs/INFRAESTRUCTURA.md) para el inventario vivo. Las referencias históricas a `sentinel-cubepath` (CubePath) y al antiguo `fenix` (GCloud) corresponden a infraestructura ya retirada.

---

## 🔬 Fundamentos Técnicos: El Paradigma S60

MycNet rechaza el uso de aritmética de punto flotante (`float`) debido a la generación de entropía (calor y bugs) por fracciones infinitas. En su lugar, utiliza **Enteros Escalados S60**:

*   **Eficiencia Matemática**: El sistema opera en Base-60 (divisores: 1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 30, 60), eliminando la pérdida de información en cálculos de frecuencia y fase.
*   **Aritmética de No-Colapso**: Implementación de la tabla de recíprocos **PAI-60** para divisiones en O(1) con precisión absoluta en el kernel.

---

## 🔱 Componentes Core

### 1. ADM (Active Mycelium Design)
Capa de transporte basada en el protocolo `batman-adv` (L2 Mesh), optimizada para:
*   **Topología Adaptativa**: Enrutamiento dinámico que emula el crecimiento y reparación del micelio biológico.
*   **Gestión de Congestión**: Uso de `fq_codel` y `cake` para mantener la latencia p95 bajo control, evitando el *bufferbloat*.
*   **Control Hexagonal**: Segmentación de la red en hexágonos de coherencia SPA (Sentinel Phase Alignment).

### 2. QHC Driver: Modulación YHWH
Motor de sincronización temporal que sustituye el reloj maestro de cuarzo por un oscilador orgánico:
*   **Patrón 10-5-6-5**: Modulación de frecuencia (Yod-He-Vav-He) para imitar ritmos naturales y compensar la deriva cuántica.
*   **Frecuencia Base**: ~41.77 Hz (residente en el "Tick Sagrado" de 23,939,835 ns).

### 3. Salto-17 y Segundo 68 (Quantum Leap)
Mecanismo de purga entrópica basado en la firma matemática de **Plimpton 322 Fila 17**:
*   **Reset de Fase**: Cada 17 ticks se ejecuta una corrección de fase localizada (~0.7ms).
*   **Consistencia Global**: Cada 68 segundos (17x4) se fuerza un reinicio de fase a 0.00 para sincronizar el estado entre nodos distribuidos.

### 4. Integración eBPF (Ring 0)
Módulos LSM (`ai_guardian`) que operan en el nivel más profundo del kernel Linux:
*   **Filtro de Resonancia**: Rechazo inmediato de paquetes o eventos con frecuencias irregulares (no 5-smooth).
*   **Zero-Copy Bridge**: Transferencia de eventos vía Ring Buffer a userspace con latencia < 1µs.

---

## 📦 Arquitectura de Almacenamiento
Estrategia híbrida de storage distribuido:
1.  **Fase Inicial (Validación)**: MinIO con Erasure Coding (EC 4+2).
2.  **Fase Avanzada (Producción)**: Ceph con **Modulación QHC** para rebalanceo de datos (Yatra Protocol).

---

## 🛠 Estructura del Repositorio

```bash
mycnet/
├── scripts/             # Deployment (batman-adv, MinIO, Ceph)
│   ├── mesh_setup.sh    # Configuración de malla por nodo
│   └── s60_monitor.py   # Instrumentación de métricas S60
├── target/              # Binarios Rust (mycnetd, cortex-bridge)
├── docs/                # Investigación de frontera y especificaciones (Drafts)
│   ├── TesisResonancia.md   # Validación estadística Base-60
│   └── PAI60_ebpf.md        # Integración de tabla de recíprocos en kernel
└── configs/             # Units de systemd y dashboards de Grafana
```

---

## 🚀 Estado de la Implementación
*   **Nodo único**: `fenix` — Azure `Standard_D4as_v6` (4 vCPU / 15 GiB), Brazil South, Ubuntu 24.04, kernel 6.17. Detalle completo en [`docs/INFRAESTRUCTURA.md`](docs/INFRAESTRUCTURA.md).
*   **Ring 0 eBPF/LSM**: **operativo** — `ai_guardian`, `guardian_cognitive`, `cortex_events`, `float_detector`, `gamma_heartbeat` pineados en `/sys/fs/bpf`.
*   **Red**: WireGuard (`wg0`, `10.88.0.1/24`) activo. Mesh `batman-adv` (`bat0`) **no desplegada** aún en el nuevo nodo.
*   **Sincronización**: `yhwh_driver` pendiente de revalidar deriva en el nuevo `fenix`.
*   **Métricas**: Instrumentación básica de TQ (Transmit Quality) mapeada a formatos SPA.

---

