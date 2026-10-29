# Medición de capacidad entre proveedores

## Alcance

La comparación usa las mismas parejas y nodos por dos rutas:

- **WireGuard L3:** tráfico IP por `wg0` (`10.88.0.0/24`), sin pasar por `bat0`.
- **Batman:** IP sobre `bat0` (`10.10.0.0/24`), con VXLAN y Batman-adv dentro de WireGuard.

La primera ronda se ejecutó con Fenix, Hercules, Ocean y laptop como spokes de Kingu; por eso esos datos no representan enlaces directos entre spokes. El 2026-10-29 se completó y validó la malla WireGuard entre los cuatro nodos cloud, con las seis aristas directas. La laptop sigue como spoke por NAT. Consultar [`DIAGNOSTICO_20261029.md`](DIAGNOSTICO_20261029.md) para distinguir resultados de topología hub de las pruebas directas posteriores. La comparación mide WireGuard L3 frente a Batman sobre WireGuard, no Internet público sin cifrar. Los archivos `benchmark_comparativo_transportes.py` y `benchmark_mycnet_transfer.py` son históricos; el primero simula la expansión fractal y no mide una transferencia MycNet real.

## Parejas predeterminadas

| Pareja | Cobertura |
|---|---|
| Fenix ↔ Kingu | Vultr ↔ Vultr |
| Kingu ↔ Hercules | Vultr ↔ GCP |
| Kingu ↔ Ocean | Vultr ↔ Linode |
| Hercules ↔ Ocean | GCP ↔ Linode |
| Fenix ↔ Hercules | Vultr ↔ GCP |
| Fenix ↔ Ocean | Vultr ↔ Linode |
| laptop ↔ Kingu | red residencial ↔ Vultr |

## Señales recogidas

- Ping por ambas direcciones: RTT medio y pérdida.
- TCP por ambas direcciones: caudal recibido y retransmisiones.
- UDP por ambas direcciones a 5 y 20 Mbps: caudal recibido, jitter y pérdida.
- Comprobación de `named` y consulta SOA en Hercules y Ocean entre parejas.
- Una sola transferencia activa a la vez; el puerto iperf es aleatorio dentro de `52000–61999`, limitado a las subredes WireGuard/Batman y retirado al terminar.

El TCP sostenido estima capacidad disponible para una transferencia larga. UDP comprueba si una tasa ofrecida causa pérdida/jitter; no sustituye a una prueba de capacidad máxima. La comparación final debe usar mediana de al menos tres rondas y realizarse en distintos horarios. Los límites de vCPU, la carga de los nodos y las rutas entre regiones/proveedores afectan el resultado.

## Ejecución

```bash
python3 scripts/benchmark_network_matrix.py --repeat 2
```

Resultados CSV en `docs/benchmarks/results/`. `iperf3` se instala previamente como herramienta manual; el benchmark no habilita un servidor persistente. Los listeners temporales se eliminan por PID y las reglas de firewall se revierten aun si una medición falla.

Para aislar una ruta, protocolo o sentido, usar por ejemplo `--pairs hercules:ocean --paths wireguard --protocols tcp --direction reverse --skip-ping --tcp-seconds 30`. Esto permite repetir el mismo flujo mientras se observan contadores del nodo de tránsito.
