# Resultados iniciales de red — 2026-10-29

Archivo de datos: [`red_20261029T081008Z.csv`](results/red_20261029T081008Z.csv) (80 registros, una ronda completa). Esta es la medición **anterior a la malla directa**; hubo retransmisiones TCP en varias parejas, detalladas abajo.

## Caudal TCP

| Pareja | WireGuard (A→B / B→A) Mbps | Batman sobre WireGuard (A→B / B→A) Mbps | Cambio promedio de Batman | RTT medio WG / Batman |
|---|---:|---:|---:|---:|
| fenix ↔ kingu | 797.7 / 918.0 | 577.8 / 710.3 | -24.9% | 0.5 / 0.6 ms |
| kingu ↔ hercules | 279.3 / 138.2 | 221.4 / 142.6 | -12.8% | 137.1 / 137.5 ms |
| kingu ↔ ocean | 327.9 / 440.0 | 278.3 / 409.2 | -10.5% | 48.2 / 48.3 ms |
| hercules ↔ ocean | 92.7 / 78.5 | 86.1 / 87.5 | +1.4% | 185.8 / 185.8 ms |
| laptop ↔ kingu | 524.9 / 403.6 | 499.1 / 328.0 | -10.9% | 17.8 / 14.5 ms |

## UDP a tasas ofrecidas

Se ofrecieron 5 y 20 Mbps por dirección, con datagramas de 1200 bytes. En las cinco parejas, la ruta Batman entregó prácticamente la tasa ofrecida; la pérdida máxima registrada fue menor de 0,05%. Esto valida esos niveles de carga, pero no determina el máximo UDP de cada enlace.

## Lectura y límites

- **Corrección de interpretación:** las rutas spoke↔spoke de esta ronda no eran enlaces directos entre proveedores. Fenix, Hercules, Ocean y laptop usaban a Kingu como peer WG, con `AllowedIPs=10.88.0.0/24`; Kingu tenía peers hoja como `/32`. Un traceroute confirmó Hercules→Kingu→Ocean por `wg0` (136,5 ms y luego 185,2 ms). El traceroute en `bat0` muestra solo Ocean porque el TTL del paquete interno no revela el salto del túnel VXLAN. Esta tabla compara IP L3 sobre WG con Batman/VXLAN bajo el hub de ese momento; no estima el rendimiento directo ni atribuye por sí sola el cambio a un overhead intrínseco. Después se formó la malla directa entre los cuatro nodos cloud; ver [`DIAGNOSTICO_20261029.md`](DIAGNOSTICO_20261029.md) y los CSV directos.
- La ronda mide una transferencia TCP de 6 segundos por sentido y ruta. Es una línea base, no una estimación estable de capacidad sostenida; los TCP retranmitieron especialmente en GCP↔Linode y laptop↔Kingu.
- Batman (`bat0`) va encapsulado en VXLAN/Batman sobre WireGuard. La comparación es frente al tráfico IP directo por WireGuard, manteniendo nodos y proveedor; no compara con Internet público sin cifrar.
- Batman tuvo desde 24,9% menos caudal TCP promedio en Vultr↔Vultr hasta 1,4% más en GCP↔Linode; la variación depende de la pareja y del sentido. En laptop↔Kingu la diferencia promedio fue de 10,9%, con retransmisiones elevadas en ambos caminos.
- Los RTT entre proveedores fueron similares por ambas rutas: Kingu↔Hercules ~137 ms, Kingu↔Ocean ~48 ms y Hercules↔Ocean ~186 ms. No hubo pérdida ICMP en esta ronda.
- Tras finalizar, la muestra instantánea de CPU de `mycnetd` fue Fenix 0,9%, Kingu 2,0%, Hercules 2,1% y Ocean 1,8%. No es una serie temporal ni atribuye consumo durante cada flujo.
- Las mediciones son una sola ronda en un horario y pueden reflejar congestión, rutas y variación de CPU. Repetir al menos tres rondas en distintos horarios y extender TCP a 30–60 s antes de tomar decisiones de dimensionamiento.
- DNS respondió con la SOA esperada antes de iniciar y entre parejas. Al cierre, `named` permaneció activo, `iperf3` no quedó habilitado como servicio, los listeners temporales terminaron y las reglas temporales desaparecieron.
- El archivo `parcial_interrumpida_20261029T080209Z.csv` corresponde a una corrida detenida por un fallo de listener y no forma parte de los resultados analizados.
- El benchmark no registró CPU por intervalo; la verificación final solo confirmó que no quedaron procesos iperf ni servicios de prueba activos.
- La prueba dirigida de 100 pings de GCP↔Linode encontró asimetría de latencia: Ocean→Hercules por WG tuvo media 191,9 ms, máximo 537,8 ms y mdev 40,0 ms; Hercules→Ocean tuvo mdev 0,9 ms. Por `bat0`, ambos sentidos tuvieron mdev ≤0,5 ms. No hubo pérdida ICMP en esa muestra.
- En una repetición TCP de 30 s de GCP↔Linode, hubo 473 retransmisiones Ocean→Hercules por WG y 292 por Batman; los sentidos Hercules→Ocean reportaron cero. El caudal fue 80,7–109,7 Mbps. UDP ofrecido a 20 Mbps tuvo 0% de pérdida salvo 0,005% en Batman Ocean→Hercules.
- `wg0` de Kingu reportó 2.571.687 descartes TX acumulados antes del diagnóstico; no aumentaron durante la muestra de pings y subieron a 2.572.363 durante la prueba sostenida. `tx_errors` quedó en 391 y la interfaz física no mostró descartes TX. Es un contador del dispositivo acumulado, sin atribución suficiente para asignar su causa a los flujos probados.
