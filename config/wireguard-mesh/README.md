# Preparación de la malla WireGuard

Estado: **malla directa completa entre los cuatro nodos cloud, validada el 2026-10-29**. Las seis aristas están instaladas: Fenix↔Kingu, Fenix↔Hercules, Fenix↔Ocean, Kingu↔Hercules, Kingu↔Ocean y Hercules↔Ocean. Kingu conserva los peers actuales y sigue disponible como ruta de administración. La laptop permanece como spoke detrás de NAT.

## Topología candidata

- Red inicial directa entre los cuatro nodos con IP pública estable: Fenix `.1`, Kingu `.2`, Hercules `.4` y Ocean `.5`.
- Laptop `.3` permanece como spoke de Kingu hasta resolver su NAT y confirmar un endpoint entrante estable. No asumir que el puerto NAT observado hoy será persistente.
- Los cuatro servidores públicos ya escuchan en UDP `51820`; en Fenix y Hercules el puerto quedó persistido en `wg0.conf` sin bajar la interfaz.
- Los peers nuevos entre Fenix, Hercules y Ocean anuncian la dirección WireGuard remota `/32` en cada extremo. Los peers preexistentes hacia Kingu se conservan: en Fenix el peer Kingu sigue anunciando `10.88.0.0/24`, mientras Kingu mantiene Fenix como `/32`. Por ello las rutas `/32` nuevas prevalecen sobre el peer amplio de Kingu en Hercules/Ocean, pero no hay failover automático al fallar una arista directa. Revertir una de esas aristas requiere retirar el peer `/32` en ambos extremos y comprobar la ruta por Kingu.
- No bajar ni reiniciar `wg0` durante el cambio. Incorporar los peers aditivamente, validar handshake/ruta/retransmisiones por pareja y probar persistencia antes de avanzar.

## Bloqueos verificados

- Fenix ahora fija `ListenPort=51820`; UFW permite el puerto. La regla cloud de Vultr no se pudo consultar desde la IP de salida de esta sesión.
- Hercules ahora fija `ListenPort=51820`; Firewalld permite UDP 51820 solo desde Kingu, Fenix y Ocean; GCP permite el puerto.
- Kingu y Ocean ejecutan en `51820`; sus firewalls locales permiten ese puerto. La API de Linode no reportó firewalls asociados. La API de Vultr denegó la consulta por restricción de IP de origen.
- Los enlaces Hercules↔Ocean, Fenix↔Hercules y Fenix↔Ocean se añadieron a ambos extremos sin bajar `wg0`; ping y traceroute verificaron ruta de un salto. No se retiraron los peers de Kingu.
- El detalle medido y los pasos de preparación están en [`DIAGNOSTICO_20261029.md`](../../docs/benchmarks/DIAGNOSTICO_20261029.md).

`peer-matrix.csv` es un inventario no secreto. No contiene claves privadas ni constituye una configuración lista para `wg setconf`. Las copias de `/etc/wireguard/wg0.conf` previas a los cambios se guardaron en cada nodo con sufijos `codex-mesh-20261029` y específicos por arista (`codex-mesh-peer-20261029`, `codex-mesh-fh-20261029`, `codex-mesh-fo-20261029`).
