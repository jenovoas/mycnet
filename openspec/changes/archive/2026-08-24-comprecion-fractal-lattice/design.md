## Context

El sistema actual de memoria distribuida en MycNet y el almacenamiento de eventos en Sentinel operan sobre sustratos aislados. MycNet implementa `HolographicSyncEngine` en `mycnet-memory` para sincronizar semillas fractales con cero payload de red, mientras que Sentinel (`me-60os-core`) almacena buffers planos en `/dev/shm` que consumen memoria lineal.

Este diseño define la unificación del contrato binario, la física de propagación fonónica sobre una malla hexagonal de 37 osciladores y el acoplamiento zero-copy entre ambos sistemas respetando la pureza sexagesimal del Protocolo YATRA ($60^4 = 12\,960\,000$).

## Goals / Non-Goals

**Goals:**
- Unificar la estructura `HolographicSeedBinary` de 32 bytes exactos como estándar universal cross-process (`#[repr(C)]`).
- Implementar la inyección focalizada y la difusión de Von Neumann en el `LiquidLattice` de 3 anillos ($N=37$ nodos) con conservación de energía estricta.
- Diseñar el ring buffer lock-free en `/dev/shm/me60os_fractal_seeds` (1024 slots $\times$ 32B = 32 KiB) para comunicación IPC atómica con latencia $< 1 \mu\text{s}$.
- Garantizar compilación estricta libre de floats (`f32`/`f64`) en todo el pipeline.

**Non-Goals:**
- No se reemplaza el almacenamiento masivo persistente en disco (MinIO / Ceph), que sigue como capa de persistencia secundaria.
- No se altera la topología de red L2 mesh (`batman-adv` / WireGuard), la cual actúa únicamente como transporte transparente de las semillas de 32 bytes.

## Decisions

### 1. Semilla Fija de 32 Bytes vs Payload Dinámico
- **Decisión**: Fijar la semilla fractal a 32 bytes constantes (`HolographicSeedBinary`).
- **Razón**: Permite la colocación estática en ring buffers de memoria compartida sin asignaciones en heap (`malloc`/`free`) y garantiza predicción de caché L1/L2.
- **Alternativas consideradas**: Protocol Buffers / bincode con longitud variable (descartado por sobrecarga de serialización y fragmentación de memoria).

### 2. Difusión Fonónica Discreta en S60
- **Decisión**: Implementar la difusión de energía con coeficientes enteros sexagesimales exactos:
  $$A_{\text{nuevo}}[i] = \frac{A_{\text{anterior}}[i] + \sum_{k \in \mathcal{N}(i)} A_{\text{anterior}}[k]}{1 + |\mathcal{N}(i)|}$$
- **Razón**: La aritmética en base $60^4$ no genera truncamientos periódicos y mantiene determinismo idéntico en cualquier arquitectura x86_64/ARM.
- **Alternativas consideradas**: Integradores continuos Runge-Kutta de 4to orden (descartado por requerir floats).

### 3. Sincronización IPC Atómica Lock-Free
- **Decisión**: Usar un ring buffer en POSIX SHM con dos punteros `AtomicU64` (`head` para el escritor Sentinel, `tail` para el lector MycNet).
- **Razón**: Elimina bloqueos de kernel (futex/mutex), garantizando latencia constante para el Ring 0.
- **Alternativas consideradas**: Sockets UNIX / Pipes (descartado por sobrecarga de copias de contexto en kernel).

## Risks / Trade-offs

- **[Riesgo: Desincronización de Fase en Malla L2]** → *Mitigación*: Inyección del pulso armónico 10-5-6-5 (YHWH) y purga entrópica Salto-17 cada 68 segundos para forzar alineación de fase a 0.00 entre nodos.
- **[Riesgo: Colisión de Semillas en el Mismo Nodo Hexagonal]** → *Mitigación*: Campo `phase_sector` (0..59) que permite hasta 60 modos ortogonales de oscilación en la misma coordenada cúbica $(q, r, s)$.
