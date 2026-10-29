# Diagnóstico de ruta y preparación de la malla — 2026-10-29

## Hallazgo principal: la WireGuard desplegada era hub-and-spoke

La inspección inicial de `wg show wg0 allowed-ips` y de los archivos activos confirmó:

| Nodo | Peers WireGuard actuales |
|---|---|
| Fenix `.1` | Kingu `.2`, `AllowedIPs=10.88.0.0/24` |
| Kingu `.2` | Fenix `.1/32`, laptop `.3/32`, Hercules `.4/32`, Ocean `.5/32` |
| laptop `.3` | Kingu `.2`, `AllowedIPs=10.88.0.0/24` |
| Hercules `.4` | Kingu `.2`, `AllowedIPs=10.88.0.0/24` |
| Ocean `.5` | Kingu `.2`, `AllowedIPs=10.88.0.0/24` |

Traceroute confirmó Hercules→Ocean por `wg0`: primer salto Kingu (`10.88.0.2`, 136,5 ms), segundo Ocean (`10.88.0.5`, 185,2 ms). La diferencia de ~48,7 ms coincide con el RTT Kingu↔Ocean. En `bat0`, traceroute solo muestra Ocean porque el TTL del paquete interno no expone el reenvío del datagrama VXLAN; el RTT siguió en ~186 ms. La malla Batman tiene VXLAN por pareja, pero su transporte WireGuard pasa por Kingu para el tráfico entre spokes.

En ese estado, Kingu era tránsito obligatorio para todos los pares spoke↔spoke y un posible cuello de botella/falla común. La primera ronda de WG L3 contra Batman no representa rutas proveedor↔proveedor directas; midió ambos encapsulados sobre esa topología hub-and-spoke.

## Estado después de la preparación

- Fenix y Hercules ahora tienen `ListenPort=51820` persistido; sus `wg0` no se bajaron ni reiniciaron. Fenix ya tenía UFW abierto en UDP 51820.
- En Hercules se persistieron reglas Firewalld para UDP 51820 limitadas a Kingu (`64.176.8.191`), Fenix (`64.176.28.233`) y Ocean (`172.233.6.214`). GCP ya permitía UDP 51820. Se conservaron archivos previos como copias de seguridad en `/etc/wireguard/`.
- Se añadieron los peers directos Hercules↔Ocean, Fenix↔Hercules y Fenix↔Ocean con `AllowedIPs` `/32` en ambas direcciones. Los peers de Kingu siguen instalados. Los cuatro nodos cloud forman las seis aristas directas: Fenix↔Kingu, Fenix↔Hercules, Fenix↔Ocean, Kingu↔Hercules, Kingu↔Ocean y Hercules↔Ocean. Una lectura posterior de `wg show allowed-ips` confirmó que los tres peers nuevos siguen instalados; ping/traceroute por pareja confirmó los enlaces añadidos. Las rutas directas `/32` tienen prioridad sobre la ruta amplia de Kingu; no hay failover automático si cae un enlace directo.
- La ruta directa pasó 20 pings por sentido, 0% pérdida y traceroute de un salto. También pasó 20 pings Batman por sentido y consultas SOA entre Hercules y Ocean.
- La prueba directa de 30 s dio:

| Ruta | Hercules→Ocean TCP | Ocean→Hercules TCP | Retransmisiones | UDP ofrecido 20 Mbps |
|---|---:|---:|---:|---:|
| WG directo | 118,9 Mbps | 161,0 Mbps | 0 / 0 | 19,7 / 20,0 Mbps; pérdida 0% |
| Batman sobre WG directo | 108,6 Mbps | 159,3 Mbps | 0 / 0 | 19,7 / 20,0 Mbps; pérdida 0% |

El RTT directo bajó de ~185 ms por Kingu a ~138 ms. Durante estas transferencias `wg0 tx_dropped` en Kingu permaneció en 2.573.882, sin nuevos descartes; antes, los flujos Ocean→Hercules por el hub habían incrementado ese contador casi 1:1 con las retransmisiones. Datos directos: [`malla_directa_hercules_ocean_20261029.csv`](results/malla_directa_hercules_ocean_20261029.csv).

- En Fenix↔Ocean directo, una muestra de 10 s midió WG TCP 334,2 Mbps Fenix→Ocean y 386,9 Mbps Ocean→Fenix; el sentido Ocean→Fenix tuvo **320 retransmisiones**. Batman TCP dio 274,3 y 421,7 Mbps, con cero retransmisiones en ambos sentidos. UDP a 5/20 Mbps tuvo 0% de pérdida en ambas rutas y sentidos. RTT fue ~48,7 ms y sin pérdida ICMP. Es una sola muestra y no identifica la causa de las retransmisiones; la asimetría persiste incluso sin tránsito por Kingu. Datos: [`malla_directa_fenix_ocean_20261029.csv`](results/malla_directa_fenix_ocean_20261029.csv).

- Al terminar esta medición, Fenix, Kingu, Hercules y Ocean reportaron `named` activo; no quedaron procesos `iperf3`. La laptop conserva WireGuard en UDP 39385 detrás de NAT y no se modificó. Los peers de Kingu permanecen instalados como rutas de administración y recuperación.

## Síntomas reproducidos

- En 100 pings, Ocean→Hercules por WG tuvo RTT medio 191,9 ms, máximo 537,8 ms y desviación (mdev) 40,0 ms. Hercules→Ocean tuvo mdev 0,9 ms. Batman tuvo mdev ≤0,5 ms en ambos sentidos; pérdida ICMP observada: 0%.
- En TCP sostenido durante 30 s entre Hercules y Ocean: WG Hercules→Ocean 88,3 Mbps y 0 retransmisiones; WG Ocean→Hercules 80,7 Mbps y 473 retransmisiones. Batman Hercules→Ocean 81,1 Mbps y 0 retransmisiones; Batman Ocean→Hercules 109,7 Mbps y 292 retransmisiones.
- Dos pruebas aisladas adicionales de 30 s en WG Ocean→Hercules registraron 1.027 y 493 retransmisiones. En Kingu, `wg0 tx_dropped` aumentó respectivamente 1.027 y 492 entre las lecturas inmediatamente anterior y posterior; `tx_errors` permaneció en 391 y el descarte TX de `enp1s0` en 0. Esta correlación casi 1:1 ubica los descartes en el tránsito WireGuard de Kingu y acompaña las retransmisiones del flujo. El código upstream de WireGuard incrementa `tx_dropped` al eliminar paquetes antiguos cuando su cola staged excede el límite; confirmar el mismo comportamiento en el kernel personalizado `7.3.0-ifenix` antes de declarar la causa raíz ([controlador upstream](https://github.com/WireGuard/wireguard-linux/blob/stable/drivers/net/wireguard/device.c)).
- Durante una muestra de 35 s que cubrió una de las pruebas, Kingu tenía 4 vCPU y no mostró saturación por núcleo (aprox. 2–4% ocupado); `mycnetd` estaba cerca de 1,6%. Esto no apunta a saturación general de CPU como explicación de los descartes, aunque no descarta pausas/colas específicas del kernel.
- UDP ofrecido a 20 Mbps se recibió cerca de esa tasa. Pérdida: 0% salvo 0,005% en Batman Ocean→Hercules.
- Kingu mostró `wg0 tx_dropped=2.571.687` y `tx_errors=391` antes de las pruebas largas. `tx_dropped` no aumentó durante la ronda de pings y llegó a 2.572.363 durante la primera prueba sostenida (+676 entre muestras); las dos pruebas aisladas posteriores elevaron el contador a 2.573.882 con los deltas descritos arriba. La interfaz física no reportó descartes TX. Los valores son acumulados y no identifican por sí solos qué peer/cola descartó cada paquete.
- `tc -s` en la interfaz física de Kingu mostró 5.626 descartes acumulados en sus colas FQ (repartidos entre cuatro colas). No se midió el delta de estas colas durante cada flujo.

Los datos sostenidos completos están en [`diagnostico_gcp_linode_20261029.csv`](results/diagnostico_gcp_linode_20261029.csv).

## Puertos y estado de la malla directa

| Nodo | Puerto WG en ejecución | Puerto previsto/permitido observado | Estado |
|---|---:|---:|---|
| Fenix | 51820/UDP | UFW permite 51820/UDP | Peers directos con Kingu, Hercules y Ocean; regla cloud de Vultr no verificada |
| Kingu | 51820/UDP | Firewalld permite 51820/UDP | Peers con los cuatro nodos; consulta de Vultr Firewall API rechazada por restricción de IP de origen |
| Hercules | 51820/UDP | Firewalld permite 51820/UDP desde sus tres peers cloud; GCP VPC permite 51820/UDP | Peers directos con Fenix, Kingu y Ocean |
| Ocean | 51820/UDP | nftables permite 51820/UDP; Linode no reporta firewalls asociados | Peers directos con Fenix, Kingu y Hercules |
| laptop | 39385/UDP observado detrás de NAT | `wg-quick@wg0` inactivo; perfil gestionado por NetworkManager | Solo peer Kingu; endpoint entrante estable no comprobado, no se modificó |

Los peers de Kingu todavía tienen endpoints configurados Fenix `:39027`, Hercules `:48533` y laptop `:37874`; en la sesión los endpoints observados por roaming eran Fenix `:39027`, Hercules `:43302` y laptop `:39385`. Los puertos variables y la dependencia de conexiones iniciadas hacia Kingu explican por qué un túnel activo no demuestra que otro peer pueda iniciar una conexión entrante.

El inventario no secreto y el estado de la malla cloud con laptop como spoke pendiente de NAT están en [`config/wireguard-mesh/`](../../config/wireguard-mesh/).

## Preparación propuesta

1. Conservar los peers de Kingu y las consolas de proveedor como rutas de administración y recuperación; no retirar el hub todavía.
2. Verificar las reglas cloud de Vultr desde una IP de salida autorizada; actualmente no se pudo consultar la API.
3. Repetir al menos tres muestras sostenidas de los seis enlaces directos, investigar específicamente las retransmisiones Ocean→Fenix y medir contadores antes/después en ambos extremos.
4. Mantener laptop como spoke a Kingu mientras dependa de NAT dinámica; incorporarla a una malla directa solo con endpoint/port-forward entrante estable y probado.
5. Probar persistencia tras reinicio en ventana coordinada y con recuperación por consola antes de considerar retirar cualquier peer hub.

Se fijaron listeners de Fenix y Hercules, se agregaron reglas UDP acotadas a peers en Hercules y peers directos de forma aditiva en los tres pares indicados; no se retiró ningún peer de Kingu ni se cambió `ip_forward`. Las reglas temporales del benchmark se retiraron; DNS siguió activo y no quedaron procesos `iperf3`.
