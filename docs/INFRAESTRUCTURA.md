# Infraestructura actual — MyCNet

> **Actualizado:** 2026-10-29. Despliegue de `mycnetd` comprobado por SSH, systemd y API en las cinco máquinas; malla WireGuard directa validada entre los cuatro nodos cloud, con laptop como spoke NAT. El estado remoto es una comprobación puntual.

## Nodos y roles

La prueba de fuego comprende cinco máquinas físicas. Los nombres `ns1`/`ns2` son nombres DNS de Hercules/Ocean, no nodos adicionales. El objetivo es que MyCNet corra en las cinco máquinas.

| Máquina | Acceso SSH comprobado | IP pública | IP WireGuard | Rol esperado | MyCNet comprobado |
|---|---|---|---|---|---|
| `hercules` | `ns1.pinguinoseguro.cl:4222` | `35.222.197.225` | `10.88.0.4` | DNS público Hercules; nombre DNS interno `ns1.pinguinoseguro.cl` | **Activo**: binario nuevo en ejecución; API `/health` HTTP 200; métricas orientadas a Kingu; UDP escucha en `:4242`. |
| `ocean` | `ns2.pinguinoseguro.cl:4222` | `172.233.6.214` | `10.88.0.5` | DNS público Ocean; nombre DNS interno `ns2.pinguinoseguro.cl` | **Activo**: binario nuevo en ejecución; API `/health` HTTP 200; UDP escucha en `:4242`. |
| `kingu` | `kingu:4222` / `kingu-vpn` | `64.176.8.191` | `10.88.0.2` | DNS interno adicional y frontend/proxy | **Activo**: binario nuevo ejecutándose tras reinicio del usuario; métrica configurada hacia Hercules. |
| `fenix` | Host local de esta sesión | `64.176.28.233` | `10.88.0.1` | Backend y DNS interno primario | **Activo**: binario nuevo instalado en disco; proceso anterior sigue en memoria hasta reinicio manual del servicio. |
| `laptop` (`ifenix`) | `10.88.0.3:22` | No comprobada | `10.88.0.3` | Nodo de la prueba MyCNet | **Activo**: binario nuevo instalado en disco; proceso anterior sigue en memoria hasta reinicio manual del servicio. |

Los destinos `ns1.pinguinoseguro.cl` y `ns2.pinguinoseguro.cl` resuelven a las IP públicas indicadas y sus máquinas reportan los hostnames `hercules` y `ocean`. Los nombres sueltos `hercules` y `ocean` no resuelven desde este entorno; usar sus FQDN para SSH. `laptop` e `ifenix` son alias de una misma máquina; SSH responde en el puerto 22 (no en 4222). Fenix es el host local de esta sesión (`hostname=fenix`, `10.88.0.1`); se inspeccionó directamente, sin SSH.

## DNS y enmascaramiento de servicios

- **Hercules / `ns1.pinguinoseguro.cl`:** `bind9` está activo y configurado como master de `pinguinoseguro.cl` y `laespiguita.cl`; tiene listeners en loopback y en la interfaz privada de nube (`10.128.0.2`). La IP pública `35.222.197.225` no aparece como dirección local de la máquina.
- **Ocean / `ns2.pinguinoseguro.cl`:** `bind9` está activo y configurado como slave de las zonas públicas con Hercules (`10.88.0.4`) como master; tiene listeners en su IP pública y WireGuard (`10.88.0.5`).
- **Kingu:** `bind9` está activo en `10.88.0.2`; es slave de las zonas públicas y de la zona interna `vpn`/reversa, con Fenix (`10.88.0.1`) como master para las zonas internas. `nginx` escucha en 80/443; el sitio `pinguinoseguro.cl` reenvía a `127.0.0.1:3000`, donde está publicado el contenedor `pinguinoseguro-web`.
- **Fenix:** BIND9 está activo y es master de `vpn` y `88.10.in-addr.arpa`, y slave de las zonas públicas con Hercules y Ocean configurados como masters. `sentinel-cortex` respondía HTTP 200 en `:8000`. PostgreSQL corría en Docker, publicado solo en `10.88.0.1:5432`.
- **Ruta frontend/backend:** Nginx de Kingu envía al contenedor local `pinguinoseguro-web` en `127.0.0.1:3000`; la configuración del contenedor indica que su host de base de datos es `10.88.0.1` (Fenix). Esto concuerda con frontend en Kingu y backend de datos en Fenix; no es un proxy HTTP directo de Nginx a Fenix.

Desde Kingu, los cuatro servidores DNS (`10.88.0.1`, `.2`, `.4` y `.5`) contestaron consultas SOA/NS para `pinguinoseguro.cl`; las respuestas A de `ns1` y `ns2` apuntaron a las IP públicas. La configuración local de BIND confirma Fenix master para `vpn`/reversa, Hercules master para las zonas públicas, y los demás como slaves según las zonas. Desde el entorno de revisión, Ocean respondió la SOA directamente; Hercules y el resolver público `1.1.1.1` no respondieron a la consulta directa en ese momento. Esto no valida transferencia efectiva de zonas, ACL ni delegación global.

## MyCNet y topología

`mycnetd` está activo y habilitado en las cinco máquinas; las cinco instalaciones comparten el hash `43e1fcfd553a5157a602d7b206702599d0426e186e93924f70fb2d40956a24f6`. El emisor UDP tiene cuatro peers configurados por nodo y escucha en `:4242`. El servicio de instantáneas `batctl` está instalado y la API reportó cuatro vecinos Batman por nodo tras su despliegue. La inspección inicial de WireGuard encontró hub-and-spoke con Kingu como tránsito obligatorio. El 2026-10-29 se agregaron peers directos y se validaron las seis aristas entre Fenix, Kingu, Hercules y Ocean; los peers de Kingu siguen instalados. La laptop (`.3`) continúa como spoke por NAT. Esta validación de transporte no demuestra por sí sola intercambio efectivo de semillas MyCNet.

El 2026-10-29 se compiló `mycnetd` en release con Rust 1.99.1 desde este checkout. La versión con métricas por peer (`a54399149b926e2a138210df520d8e94364edc64ae550865683e7e45a1fdfadc`) está instalada en disco en los cinco nodos; hay copias previas con sufijo `predeploy-20261029-metrics-peer`. Hercules, Ocean y Kingu ya ejecutan esa versión tras reinicios del usuario. Fenix y laptop siguen ejecutando el proceso anterior hasta que el usuario reinicie `mycnetd`. Se configuró `MYCNET_METRICS_PEER=10.88.0.4:4242` en Kingu y `10.88.0.2:4242` en Hercules; ambos daemons ya tienen esa configuración activa.

En las últimas consultas, Kingu y Hercules no tienen `/dev/shm/me60os_lattice` ni `/dev/shm/me60os_fractal_seeds`; sus daemons reportan `fractal_source_connected=false`. Kingu tenía los buffers antes de su reinicio reciente, pero no se recrearon al arrancar; no se encontró un proceso o unidad Sentinel/Cortex activo en Kingu. Fenix, laptop y Ocean también carecían de esos buffers en la revisión anterior. Según `poll_local_fractal_seeds()` en `crates/mycnet-daemon/src/state.rs`, sin el puente fractal cada sondeo devuelve una lista vacía; MycNet no tiene semillas reales que emitir. La instrumentación está activa en Kingu y Hercules, pero los contadores siguen en cero: el benchmark real está bloqueado hasta recuperar el ring SHM productor.

La configuración de peers leída en los servicios es:

| Nodo | Peers configurados |
|---|---|
| Fenix (`.1`) | `.2`, `.3`, `.4`, `.5` |
| Kingu (`.2`) | `.1`, `.3`, `.4`, `.5` |
| laptop (`.3`) | `.1`, `.2`, `.4`, `.5` |
| Hercules (`.4`) | `.1`, `.2`, `.3`, `.5` |
| Ocean (`.5`) | `.1`, `.2`, `.3`, `.4` |

El 2026-10-29 se desplegó `batman-adv` en los cinco nodos. Cada pareja usa un VXLAN unicast con VNI propio sobre `wg0`; los diez túneles usan UDP 4789 dentro de WireGuard. `bat0` tiene `10.10.0.1`–`10.10.0.5/24` según el nodo y cuatro interfaces VXLAN activas por host. Fenix, Hercules y Ocean permiten UDP 4789 desde sus cuatro peers mediante reglas específicas; Kingu mantiene `wg0` en la zona `trusted`. Laptop no tiene un cortafuegos local activo; sus túneles VXLAN fijan las cuatro direcciones remotas y viajan dentro de WireGuard.

La tabla `batctl n` informa cuatro vecinos directos por nodo y `batctl o` muestra sus rutas/TQ. Se corrigió el analizador Rust para unir vecinos directos con su TQ de la ruta preferida. Como `mycnetd` no necesita `CAP_NET_ADMIN`, un servicio aislado con solo esa capacidad captura `batctl n/o` una vez por segundo en `/run/mycnet/batctl-originators`; el daemon de usuario lee la instantánea y la API reporta cuatro vecinos. `bat_ip` ahora apunta a `10.10.0.x` también en laptop y Ocean.

La formación de vecinos Batman y la conectividad IP de extremo a extremo por `bat0` quedaron comprobadas antes de los reinicios recientes: se enviaron 10 pings por cada pareja dirigida entre los cinco nodos, con 0% de pérdida. Tras reiniciar Hercules y Ocean, `mycnet-batman.service` falló porque se ejecutó antes de que existiera `wg0`. Se añadió `After=wg-quick@wg0.service`, se recargó systemd sin reiniciar nodos y se iniciaron los servicios Batman fallidos; luego Hercules↔Ocean respondió 10/10 pings en ambas direcciones, después de una primera convergencia ARP fallida. La corrección quedó en `systemd/mycnet-batman.service` y se instaló en los cinco hosts.

### Criterio de cierre de la prueba de fuego

- `mycnetd` instalado, habilitado y saludable en las cinco máquinas físicas; **comprobado el 2026-10-29**.
- Peers configurados para los cinco nodos, incluida `laptop`; cuatro vecinos Batman visibles por nodo. El intercambio UDP de semillas sigue pendiente de validar cuando existan buffers SHM con datos.
- Camino de frontend en Kingu hacia PostgreSQL en Fenix ya observado; comprobar salud de la aplicación y las rutas de enmascaramiento de servicios.
- DNS público e interno validado para las zonas y registros previstos desde redes externas e internas.
- Puentes Sentinel probados independientemente. En esta comprobación, Kingu y Hercules conectaron ambos buffers SHM; Fenix, laptop y Ocean no los tenían. No asumir que Ring 0 de Sentinel activa los puentes de MyCNet.

## Componentes de red y almacenamiento aún no verificados

`bat0`, VXLAN y el descubrimiento Batman están desplegados y comprobados en las cinco instalaciones MycNet. MinIO y Prometheus/Grafana no se confirmaron. `docker-compose.yml` describe servicios, pero no constituye evidencia de despliegue.

## Pruebas de rendimiento de red

La metodología reproducible y el benchmark están en [`docs/benchmarks/METODOLOGIA_RED.md`](benchmarks/METODOLOGIA_RED.md) y [`scripts/benchmark_network_matrix.py`](../scripts/benchmark_network_matrix.py). Los resultados iniciales y su limitación de topología están en [`docs/benchmarks/RESULTADOS_20261029.md`](benchmarks/RESULTADOS_20261029.md); el traceroute, las retransmisiones y los bloqueos de puertos para preparar peers directos están en [`docs/benchmarks/DIAGNOSTICO_20261029.md`](benchmarks/DIAGNOSTICO_20261029.md). Los datos completos están en `docs/benchmarks/results/`.

## Infraestructura histórica

| Recurso o diseño | Estado |
|---|---|
| `sentinel-cubepath` / `vps23309.cubepath.net` | Retirado tras el hackatón CubePath 2026. |
| Terraform CubePath (`terraform/mycnet_nodes.tf`) | Diseño histórico; no describe ni aprovisiona esta infraestructura. No ejecutar para la prueba actual. |
| Guías `4_NODE_*` y escenarios `n1`–`n6` | Procedimientos de laboratorio anteriores; no representan el inventario de cinco máquinas actual. |

## Implementación MyCNet en el repositorio

El repositorio contiene `HolographicSyncEngine` (`crates/mycnet-memory/src/holographic_sync.rs`), `HolographicSeed` y mecanismos de expansión. La presencia de estas capacidades en el código no demuestra sincronización correcta entre nodos desplegados.
