#!/bin/sh
set -eu

snapshot_dir=/run/mycnet
snapshot_file="$snapshot_dir/batctl-originators"
vxlan_file="$snapshot_dir/vxlan-remotes"
tmp_file=$(mktemp "$snapshot_dir/.originators.XXXXXX")
tmp_vxlan=$(mktemp "$snapshot_dir/.vxlan-remotes.XXXXXX")
tmp_links=$(mktemp "$snapshot_dir/.vxlan-links.XXXXXX")
trap 'rm -f "$tmp_file" "$tmp_vxlan" "$tmp_links"' EXIT HUP INT TERM

/usr/sbin/batctl n > "$tmp_file"
printf '\n---MYCNET-ORIGINATORS---\n' >> "$tmp_file"
/usr/sbin/batctl o >> "$tmp_file"
chmod 0644 "$tmp_file"
mv -f "$tmp_file" "$snapshot_file"

# Export only real configured VXLAN remotes reported by the kernel. The file
# contract is "interface IPv4/IPv6" per line, consumed by tq_for_peer_ip.
/usr/sbin/ip -d -o link show type vxlan > "$tmp_links"
awk '{
    iface = $2
    sub(/:$/, "", iface)
    sub(/@.*/, "", iface)
    for (i = 1; i < NF; i++) {
        if ($i == "remote") {
            print iface, $(i + 1)
            break
        }
    }
}' "$tmp_links" > "$tmp_vxlan"
chmod 0644 "$tmp_vxlan"
mv -f "$tmp_vxlan" "$vxlan_file"
