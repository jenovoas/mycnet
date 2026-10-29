#!/bin/sh
set -eu

snapshot_dir=/run/mycnet
snapshot_file="$snapshot_dir/batctl-originators"
tmp_file=$(mktemp "$snapshot_dir/.originators.XXXXXX")
trap 'rm -f "$tmp_file"' EXIT HUP INT TERM

/usr/sbin/batctl n > "$tmp_file"
printf '\n---MYCNET-ORIGINATORS---\n' >> "$tmp_file"
/usr/sbin/batctl o >> "$tmp_file"
chmod 0644 "$tmp_file"
mv -f "$tmp_file" "$snapshot_file"
