# Infraestructura actual — MyCNet

> **Actualizado:** 2026-10-27. Inventario observado por SSH y servicios; los roles esperados proceden de la arquitectura indicada por el usuario. El estado remoto es una comprobación puntual.

## Nodos y roles

La prueba de fuego comprende cinco máquinas físicas. Los nombres `ns1`/`ns2` son nombres DNS de Hercules/Ocean, no nodos adicionales. El objetivo es que MyCNet corra en las cinco máquinas.

| Máquina | Acceso SSH comprobado | IP pública | IP WireGuard | Rol esperado | MyCNet comprobado |
|---|---|---|---|---|---|
| `hercules` | `ns1.pinguinoseguro.cl` | `35.222.197.225` | `10.88.0.4` | DNS público Hercules; nombre DNS interno `ns1.pinguinoseguro.cl` | **Activo**: servicio habilitado; API responde en `10.88.0.4:7474`; UDP configurado en `:4242`. |
| `ocean` | `ns2.pinguinoseguro.cl` | `172.233.6.214` | `10.88.0.5` | DNS público Ocean; nombre DNS interno `ns2.pinguinoseguro.cl` | **Activo**: servicio habilitado; API responde en `10.88.0.5:7474`; UDP configurado en `:4242`. |
| `kingu` | `kingu` / `kingu-vpn` | `64.176.8.191` | `10.88.0.2` | DNS interno adicional y frontend/proxy | **Activo**: servicio habilitado; API responde en `10.88.0.2:7474`; UDP configurado en `:4242`. |
| `fenix` | Host local de esta sesión | `64.176.28.233` | `10.88.0.1` | Backend y DNS interno primario | **Activo**: servicio habilitado; `/health` responde HTTP 200; escucha en `10.88.0.1:7474`; UDP configurado en `:4242`. |
| `laptop` (`ifenix`) | `laptop` / `ifenix` | No comprobada | `10.88.0.3` | Nodo de la prueba MyCNet | **Incompleto**: binario `~/.local/bin/mycnetd` no encontrado en la sesión remota; no hay unidad ni proceso activo. |

Los destinos `ns1.pinguinoseguro.cl` y `ns2.pinguinoseguro.cl` resuelven a las IP públicas indicadas y sus máquinas reportan los hostnames `hercules` y `ocean`. Los nombres sueltos `hercules` y `ocean` no resuelven desde este entorno; usar sus FQDN para SSH. `laptop` e `ifenix` son alias de una misma máquina. Fenix es el host local de esta sesión (`hostname=fenix`, `10.88.0.1`); se inspeccionó directamente, sin SSH.

## DNS y enmascaramiento de servicios

- **Hercules / `ns1.pinguinoseguro.cl`:** `bind9` está activo y configurado como master de `pinguinoseguro.cl` y `laespiguita.cl`; tiene listeners en loopback y en la interfaz privada de nube (`10.128.0.2`). La IP pública `35.222.197.225` no aparece como dirección local de la máquina.
- **Ocean / `ns2.pinguinoseguro.cl`:** `bind9` está activo y configurado como slave de las zonas públicas con Hercules (`10.88.0.4`) como master; tiene listeners en su IP pública y WireGuard (`10.88.0.5`).
- **Kingu:** `bind9` está activo en `10.88.0.2`; es slave de las zonas públicas y de la zona interna `vpn`/reversa, con Fenix (`10.88.0.1`) como master para las zonas internas. `nginx` escucha en 80/443; el sitio `pinguinoseguro.cl` reenvía a `127.0.0.1:3000`, donde está publicado el contenedor `pinguinoseguro-web`.
- **Fenix:** BIND9 está activo y es master de `vpn` y `88.10.in-addr.arpa`, y slave de las zonas públicas con Hercules y Ocean configurados como masters. `sentinel-cortex` respondía HTTP 200 en `:8000`. PostgreSQL corría en Docker, publicado solo en `10.88.0.1:5432`.
- **Ruta frontend/backend:** Nginx de Kingu envía al contenedor local `pinguinoseguro-web` en `127.0.0.1:3000`; la configuración del contenedor indica que su host de base de datos es `10.88.0.1` (Fenix). Esto concuerda con frontend en Kingu y backend de datos en Fenix; no es un proxy HTTP directo de Nginx a Fenix.

Desde Kingu, los cuatro servidores DNS (`10.88.0.1`, `.2`, `.4` y `.5`) contestaron consultas SOA/NS para `pinguinoseguro.cl`; las respuestas A de `ns1` y `ns2` apuntaron a las IP públicas. La configuración local de BIND confirma Fenix master para `vpn`/reversa, Hercules master para las zonas públicas, y los demás como slaves según las zonas. Desde el entorno de revisión, Ocean respondió la SOA directamente; Hercules y el resolver público `1.1.1.1` no respondieron a la consulta directa en ese momento. Esto no valida transferencia efectiva de zonas, ACL ni delegación global.

## MyCNet y topología

Se comprobó `mycnetd` activo y habilitado en Fenix (`10.88.0.1`), Kingu (`10.88.0.2`), Hercules (`10.88.0.4`) y Ocean (`10.88.0.5`). Los cuatro endpoints `/health` responden HTTP 200. En los cuatro nodos, `/api/v1/mesh` devuelve únicamente el nodo local y `neighbors: []`; el valor `nodes: 7` del endpoint de salud no demuestra que haya siete nodos físicos conectados ni una malla funcional. Se observaron handshakes WireGuard recientes, pero `bat0` no existe en ninguno de los cuatro.

El 2026-10-27 se compiló `mycnetd` en release desde este checkout y se instaló/reinició secuencialmente en Fenix, Kingu, Hercules y Ocean. Los cuatro servicios quedaron activos; todos responden HTTP 200 en `/health` y tienen el mismo SHA-256 `43e1fcfd553a5157a602d7b206702599d0426e186e93924f70fb2d40956a24f6`. Se conservaron las versiones previas como `~/.local/bin/mycnetd.predeploy-20261027-*`. No se compiló en los nodos pequeños: Hercules y Ocean tienen aproximadamente 1 GiB de RAM; después del reinicio sus RSS eran aproximadamente 7,6 y 6,8 MiB, respectivamente.

Los cuatro nodos carecen de `/dev/shm/me60os_lattice` y `/dev/shm/me60os_fractal_seeds`; los registros confirman ambos puentes Sentinel no disponibles y ejecución standalone. Según `poll_local_fractal_seeds()` en `crates/mycnet-daemon/src/state.rs`, sin el puente fractal cada sondeo devuelve una lista vacía; el emisor UDP no tiene semillas que difundir. Por ello el despliegue del daemon está sano, pero el transporte de semillas no quedó validado ni operativo en esta inspección.

La configuración de peers leída en los servicios es:

| Nodo | Peers configurados |
|---|---|
| Fenix (`.1`) | `.2`, `.4`, `.5` |
| Kingu (`.2`) | `.1`, `.4`, `.5` |
| Hercules (`.4`) | `.1`, `.2`, `.5` |
| Ocean (`.5`) | `.1`, `.2`, `.4` |

Las cuatro unidades revisadas se configuran como peers mutuos, pero omiten `laptop` (`.3`). La respuesta vacía de vecinos en la API de Fenix indica que falta comprobar el intercambio de datos y la formación real de la malla.

### Criterio de cierre de la prueba de fuego

- `mycnetd` instalado, habilitado y saludable en las cinco máquinas físicas; falta desplegarlo en `laptop`.
- Peers configurados para los cinco nodos, incluida `laptop`, y comunicación UDP/API verificada entre cada par requerido.
- Camino de frontend en Kingu hacia PostgreSQL en Fenix ya observado; comprobar salud de la aplicación y las rutas de enmascaramiento de servicios.
- DNS público e interno validado para las zonas y registros previstos desde redes externas e internas.
- Puentes Sentinel probados independientemente. Los registros revisados anteriormente en Kingu advertían que sus buffers SHM no estaban disponibles; no asumir que Ring 0 de Sentinel activa los puentes de MyCNet.

## Componentes de red y almacenamiento aún no verificados

Se comprobó que `bat0` no existe en Fenix, Kingu, Hercules ni Ocean. No se confirmó VXLAN, MinIO ni Prometheus/Grafana. `docker-compose.yml` describe servicios, pero no constituye evidencia de despliegue.

## Infraestructura histórica

| Recurso o diseño | Estado |
|---|---|
| `sentinel-cubepath` / `vps23309.cubepath.net` | Retirado tras el hackatón CubePath 2026. |
| Terraform CubePath (`terraform/mycnet_nodes.tf`) | Diseño histórico; no describe ni aprovisiona esta infraestructura. No ejecutar para la prueba actual. |
| Guías `4_NODE_*` y escenarios `n1`–`n6` | Procedimientos de laboratorio anteriores; no representan el inventario de cinco máquinas actual. |

## Implementación MyCNet en el repositorio

El repositorio contiene `HolographicSyncEngine` (`crates/mycnet-memory/src/holographic_sync.rs`), `HolographicSeed` y mecanismos de expansión. La presencia de estas capacidades en el código no demuestra sincronización correcta entre nodos desplegados.
