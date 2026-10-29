#!/usr/bin/env bash
set -Eeuo pipefail

# Batman-adv sobre VXLAN unicast, transportado por wg0. Cada pareja usa un VNI
# exclusivo para que los diez túneles punto a punto convivan en el mismo host.
source /etc/mycnet/batman.env

BAT_IF="bat0"
WG_IF="wg0"
BAT_MTU="1280"
VXLAN_MTU="1330"

cleanup() {
    ip link del "$BAT_IF" 2>/dev/null || true
    for entry in $MESH_PEERS; do
        IFS=: read -r peer_id _peer_ip _vni <<< "$entry"
        ip link del "vxmc${peer_id}" 2>/dev/null || true
    done
}

up() {
    ip link show "$WG_IF" >/dev/null
    modprobe batman-adv
    ip link show "$BAT_IF" >/dev/null 2>&1 || ip link add "$BAT_IF" type batadv

    for entry in $MESH_PEERS; do
        IFS=: read -r peer_id peer_ip vni <<< "$entry"
        vx_if="vxmc${peer_id}"
        if ! ip link show "$vx_if" >/dev/null 2>&1; then
            ip link add "$vx_if" type vxlan id "$vni" local "$WG_IP" \
                remote "$peer_ip" dstport 4789 dev "$WG_IF"
        fi
        ip link set dev "$vx_if" mtu "$VXLAN_MTU" up
        if ! ip -o link show dev "$vx_if" | grep -q 'master bat0'; then
            batctl meshif "$BAT_IF" interface add "$vx_if"
        fi
    done

    ip link set dev "$BAT_IF" mtu "$BAT_MTU" up
    ip address replace "$MESH_IP/24" dev "$BAT_IF"
}

case "${1:-up}" in
    up) up ;;
    down) cleanup ;;
    *) echo "uso: $0 {up|down}" >&2; exit 2 ;;
esac
