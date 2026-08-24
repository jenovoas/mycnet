# 🌌 OPENSPEC-001: COMPRESIÓN FRACTAL SOBRE MALLA RESONANTE (LIQUID LATTICE S60)

```yaml
openspec: "1.0.0"
metadata:
  id: "OPENSPEC-MYC-001"
  title: "Compresión Fractal y Sincronización Holográfica sobre Liquid Lattice"
  version: "1.0.0"
  status: "APPROVED_SPECIFICATION"
  domain: "S60-Resonance / Ring-0 / Zero-Copy Memory / Mesh-Networking"
  authors:
    - name: "Jaime Novoa Sepúlveda"
      role: "Arquitecto Jefe — Protocolo YATRA / Sentinel / MycNet"
  license: "Apache-2.0 WITH Non-Commercial-Clause"
  reference_implementations:
    mycnet: "crates/mycnet-memory/src/holographic_sync.rs"
    sentinel: "sentinel/me-60os-core/src/liquid_memory.rs"
```

---

## 1. RESUMEN EJECUTIVO Y OBJETIVO

La **Compresión Fractal sobre Liquid Lattice** es un estándar de representación y transporte de memoria basado en física fonónica y aritmética sexagesimal pura ($S60$). 

### El Problema de la Memoria Tradicional (Entropía Decimal y Transferencia Lineal)
En los sistemas clásicos, almacenar o sincronizar un estado requiere buffers lineales (megabytes o gigabytes) transmitidos por la red o volcados a disco. Esto genera saturación de bus, latencia no determinista y degradación térmica. Además, el uso de punto flotante (`float`) introduce errores acumulativos periódicos.

### La Solución Holográfica
En lugar de mover datos masivos, el sistema proyecta la información como una **Semilla Fractal (`HolographicSeed`) de 32 bytes exactos** que excita un nodo en un **Tejido Resonante Hexagonal (`LiquidLattice`)**. 

La dinámica del medio (difusión fonónica de Von Neumann modulada por el pulso 10-5-6-5 YHWH) expande localmente el campo de ondas estacionarias en cada nodo:
$$\text{Estado Reconstruido} = f^{N}(\text{Semilla Fractal})$$

Esto permite:
1. **Cero payload de red** en sincronización distribuida (MycNet).
2. **Cero fragmentación de RAM** en el pipeline cognitivo neuromórfico (Sentinel).
3. **Determinismo absoluto bit-a-bit** bajo la escala sexagesimal $\text{SCALE\_0} = 60^4 = 12\,960\,000$.

---

## 2. TOPOLOGÍA Y FÍSICA DE LA MALLA RESONANTE (LATTICE)

```
                       (0, -1, 1)
                     ⬡ ─── ⬡ ─── ⬡
                    / \   / \   / \
        (-1, 0, 1) ⬡ ─── ⬡ ─── ★ ─── ⬡  (1, 0, -1)   <── Inyección Semilla
                    \ /   \ /   \ /   \                   en (q, r, s)
                     ⬡ ─── ⬡ ─── ⬡ ─── ⬡
                            (0, 1, -1)

               Evolución Fonónica: Difusión Von Neumann (S60)
```

### 2.1 Coordenadas Cúbicas Hexagonales
El Lattice se indexa mediante coordenadas cúbicas enteras $(q, r, s)$ sujetas a la restricción espacial invariante:
$$q + r + s = 0$$

Para un Lattice de $R = 3$ anillos concéntricos, el número exacto de osciladores es:
$$N_{\text{nodos}} = 1 + 3 R (R + 1) = 1 + 3(3)(4) = 37 \text{ osciladores}$$

### 2.2 Ecuación de Difusión Fonónica Discreta (Von Neumann)
En cada tick temporal (Tick Sagrado $T_0 = 23\,939\,835\text{ ns} \approx 41.77\text{ Hz}$), la energía de amplitud $A$ se propaga a los vecinos adyacentes de forma atómica:
$$A_{\text{nuevo}}[i] = \frac{A_{\text{anterior}}[i] + \sum_{k \in \mathcal{N}(i)} A_{\text{anterior}}[k]}{1 + |\mathcal{N}(i)|}$$

- **Conservación de Energía**: La suma global $\sum_i A[i]$ se conserva en cada iteración dentro del margen de discretización sexagesimal.
- **Canal Dual de Fase**:
  - **Canal A**: Amplitud escalar de energía en unidades SPA ($60^4$).
  - **Canal B**: Sector de fase $\phi \in [0, 255]$ modulado armónicamente por el ciclo 10-5-6-5.

---

## 3. ESPECIFICACIÓN BINARIA (CONTRATO BINARIO `#[repr(C)]`)

Para garantizar interoperabilidad **zero-copy** entre procesos de Rust, módulos eBPF de kernel y memoria compartida POSIX (`/dev/shm`), las estructuras deben cumplir alineación fija sin padding indefinido.

### 3.1 Semilla Fractal (`HolographicSeed` — 32 Bytes Exactos)

```
Offset (bytes)  Tamaño   Campo            Tipo          Descripción
0x00            4        token_id         u32 (LE)      ID del token o evento proyectado
0x04            4        coord_q          i32 (LE)      Coordenada q hexagonal
0x08            4        coord_r          i32 (LE)      Coordenada r hexagonal
0x0C            4        coord_s          i32 (LE)      Coordenada s hexagonal (q+r+s=0)
0x10            8        amplitude_s60    i64 (LE)      Amplitud SPA escalada (x 12,960,000)
0x18            1        phase_sector     u8            Sector de fase armónica (0..59)
0x19            1        yhwh_pulse       u8            Fase del pulso YHWH (10-5-6-5)
0x1A            2        flags            u16 (LE)      Bit 0: Activo, Bit 1: Snapped, Bit 2: Bridge
0x1C            2        crc16            u16 (LE)      Checksum CRC16-CCITT de validación
0x1E            2        reserved         [u8; 2]       Alineación a 32 bytes
```

### 3.2 Declaración en Rust
```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HolographicSeedBinary {
    pub token_id: u32,
    pub coord_q: i32,
    pub coord_r: i32,
    pub coord_s: i32,
    pub amplitude_s60: i64,
    pub phase_sector: u8,
    pub yhwh_pulse: u8,
    pub flags: u16,
    pub crc16: u16,
    pub reserved: [u8; 2],
}
```

---

## 4. CONTRATO IPC ZERO-COPY (`/dev/shm`)

El acoplamiento entre el núcleo cognitivo de Sentinel (`me-60os-core`) y el motor de transporte de MycNet (`mycnet-memory`) se realiza mediante dos buffers circulares atómicos en memoria compartida:

```
                  SENTINEL (me-60os-core)
                            │
               (Escribe Semillas Fractales)
                            ▼
    ┌──────────────────────────────────────────────────┐
    │  POSIX SHM: /dev/shm/me60os_fractal_seeds         │
    │  ┌────────────────────────────────────────────┐  │
    │  │ Header: magic(4B) + head(8B) + tail(8B)    │  │
    │  ├────────────────────────────────────────────┤  │
    │  │ RingBuffer: 1024 slots × 32B = 32,768B     │  │
    │  └────────────────────────────────────────────┘  │
    └───────────────────────┬──────────────────────────┘
                            │
               (Lee sin copias de memoria)
                            ▼
                   MYCNET (mycnet-memory)
                            │
                (Propaga vía Mesh bat0 / WG)
```

1. **Nombre del Segmento**: `/me60os_fractal_seeds`
2. **Capacidad**: 1024 semillas en anillo circular ($32\text{ KiB} + 64\text{ B header}$).
3. **Sincronización**: Punteros `head` y `tail` gestionados mediante atómicos de 64 bits (`AtomicU64` con orden `Acquire`/`Release`), sin mutex ni bloqueos de kernel.

---

## 5. MÁQUINA DE ESTADOS Y PIPELINE DE PROCESAMIENTO

### 5.1 Pipeline en Sentinel (`me-60os-core`)
```
[Evento eBPF Ring 0 / Token Cognitivo]
                 │
                 ▼
      [HolographicCompressor]
(Mapea vector/evento a Coordenada Hexagonal + Amplitud S60)
                 │
                 ▼
        [Inyección en Lattice]
  (LiquidLattice::inject_fractal)
                 │
                 ├──────────────────────────────┐
                 ▼                              ▼
     [Difusión Fonónica]             [Depósito en SHM Bridge]
 (Evolución temporal local)        (/me60os_fractal_seeds)
                 │                              │
                 ▼                              ▼
     [Alimentación SNN LIF]           [Consumo por MycNet]
 (neural_memory::LIFNeuron)        (Difusión por Malla Mesh)
```

### 5.2 Pipeline en MycNet (`mycnet-memory`)
```
    [Lectura desde SHM o Paquete Mesh bat0]
                 │
                 ▼
       [HolographicSyncEngine]
(Verifica CRC16 y decodifica Semilla 32B)
                 │
                 ▼
      [Inyección en LiquidLattice]
                 │
                 ▼
 [Reconstrucción Fonónica O(1) del Estado]
```

---

## 6. INVARIANTES Y CRITERIOS DE ACEPTACIÓN (VERIFICACIÓN)

Toda implementación que cumpla esta especificación debe superar la siguiente matriz de pruebas formales en `cargo test`:

| ID Prueba | Invariante Evaluada | Criterio de Éxito |
|---|---|---|
| `TEST-FRACTAL-001` | **Tamaño Binario Exacto** | `std::mem::size_of::<HolographicSeedBinary>() == 32` exacto. |
| `TEST-FRACTAL-002` | **Invariante Cúbica** | Para toda semilla válida: $q + r + s == 0$. |
| `TEST-FRACTAL-003` | **Conservación Fonónica** | La energía total $\sum |A_i|$ tras $N=68$ difusiones se conserva dentro del delta sexagesimal $\Delta < 1 / 60^4$. |
| `TEST-FRACTAL-004` | **Fidelidad de Reconstrucción** | Inyectar semilla en Nodo A $\implies$ Reconstruir en Nodo B genera identidad de bytes idéntica al 100%. |
| `TEST-FRACTAL-005` | **Pureza YATRA (Sin Floats)** | Cero instrucciones `f32`/`f64` en el pipeline de difusión y compresión. |

---

*OpenSpec aprobada bajo las normas del Protocolo YATRA y Sentinel Ring-0.*
