#!/usr/bin/env python3
"""Compara WireGuard L3 con la ruta MycNet Batman (`bat0`) sin flujos paralelos."""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import os
import re
import shlex
import subprocess
import sys
import time
from pathlib import Path


NODES = {
    "fenix": {"wg": "10.88.0.1", "bat": "10.10.0.1", "provider": "Vultr"},
    "kingu": {"wg": "10.88.0.2", "bat": "10.10.0.2", "provider": "Vultr"},
    "laptop": {"wg": "10.88.0.3", "bat": "10.10.0.3", "provider": "residencial"},
    "hercules": {"wg": "10.88.0.4", "bat": "10.10.0.4", "provider": "GCP"},
    "ocean": {"wg": "10.88.0.5", "bat": "10.10.0.5", "provider": "Linode"},
}

# Cubre los seis enlaces de la malla entre cuatro nodos cloud y la ruta de laptop.
PAIRS = [
    ("fenix", "kingu"),
    ("kingu", "hercules"),
    ("kingu", "ocean"),
    ("hercules", "ocean"),
    ("fenix", "hercules"),
    ("fenix", "ocean"),
    ("laptop", "kingu"),
]
PORT = 52000 + (os.getpid() % 10000)
RUN_MARKER = f"mycnetbench{os.getpid()}"
FIREWALL_CLEANUPS: list[tuple[str, list[str]]] = []
ROWS: list[dict[str, object]] = []


def remote(host: str, command: list[str], timeout: int = 20) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["ssh", "-o", "BatchMode=yes", host, shlex.join(command)],
        text=True,
        capture_output=True,
        timeout=timeout,
    )


def require_remote(host: str, command: list[str]) -> str:
    result = remote(host, command)
    if result.returncode:
        raise RuntimeError(f"{host}: {' '.join(command)}: {result.stderr.strip()}")
    return result.stdout.strip()


def add_firewall_rule(host: str, add: list[str], remove: list[str]) -> None:
    FIREWALL_CLEANUPS.append((host, remove))
    require_remote(host, add)


def open_test_port() -> None:
    """Abre el puerto solo en las subredes privadas y registra cada reversión."""
    protocols = ("tcp", "udp")
    for interface, subnet in (("wg0", "10.88.0.0/24"), ("bat0", "10.10.0.0/24")):
        for protocol in protocols:
            add_firewall_rule(
                "fenix",
                ["sudo", "ufw", "allow", "in", "on", interface, "from", subnet,
                 "to", "any", "port", str(PORT), "proto", protocol],
                ["sudo", "ufw", "--force", "delete", "allow", "in", "on", interface,
                 "from", subnet, "to", "any", "port", str(PORT), "proto", protocol],
            )

    for zone, subnet in (("wgmesh", "10.88.0.0/24"), ("public", "10.10.0.0/24")):
        for protocol in protocols:
            rule = (
                f'rule family="ipv4" source address="{subnet}" '
                f'port port="{PORT}" protocol="{protocol}" accept'
            )
            add_firewall_rule(
                "hercules",
                ["sudo", "firewall-cmd", f"--zone={zone}", f"--add-rich-rule={rule}"],
                ["sudo", "firewall-cmd", f"--zone={zone}", f"--remove-rich-rule={rule}"],
            )

    # Kingu ya admite todo wg0 por su zona trusted; solo se abre el bat0 privado.
    for protocol in protocols:
        rule = (
            f'rule family="ipv4" source address="10.10.0.0/24" '
            f'port port="{PORT}" protocol="{protocol}" accept'
        )
        add_firewall_rule(
            "kingu",
            ["sudo", "firewall-cmd", f"--zone=public", f"--add-rich-rule={rule}"],
            ["sudo", "firewall-cmd", f"--zone=public", f"--remove-rich-rule={rule}"],
        )

    # Ocean usa nftables; las reglas son solo de runtime y llevan etiqueta propia.
    for interface, subnet in (("wg0", "10.88.0.0/24"), ("bat0", "10.10.0.0/24")):
        for protocol in protocols:
            require_remote(
                "ocean",
                ["sudo", "nft", "add", "rule", "inet", "filter", "input",
                 "iifname", interface, "ip", "saddr", subnet,
                 f"{protocol}", "dport", str(PORT), "accept", "comment", RUN_MARKER],
            )


def close_test_port() -> None:
    for host, command in reversed(FIREWALL_CLEANUPS):
        result = remote(host, command)
        if result.returncode:
            print(f"AVISO: no se pudo revertir regla en {host}: {result.stderr.strip()}", file=sys.stderr)

    try:
        rules = remote("ocean", ["sudo", "nft", "-a", "list", "chain", "inet", "filter", "input"])
        handles = re.findall(rf'comment "{RUN_MARKER}" # handle (\d+)', rules.stdout)
        for handle in reversed(handles):
            result = remote("ocean", ["sudo", "nft", "delete", "rule", "inet", "filter", "input", "handle", handle])
            if result.returncode:
                print(f"AVISO: no se pudo retirar regla nft handle {handle}: {result.stderr.strip()}", file=sys.stderr)
    except Exception as exc:  # mantener visible cualquier limpieza incompleta
        print(f"AVISO: limpieza nft de Ocean incompleta: {exc}", file=sys.stderr)


def check_dns() -> None:
    for host in ("hercules", "ocean"):
        require_remote(host, ["systemctl", "is-active", "named"])
        answer = require_remote(
            host,
            ["dig", "+time=2", "+tries=1", "@127.0.0.1", "pinguinoseguro.cl", "SOA", "+short"],
        )
        if "2026102901" not in answer:
            raise RuntimeError(f"DNS {host} no devolvió la SOA esperada: {answer}")


def write_row(row: dict[str, object], output: Path) -> None:
    ROWS.append(row)
    output.parent.mkdir(parents=True, exist_ok=True)
    new_file = not output.exists()
    with output.open("a", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(row.keys()))
        if new_file:
            writer.writeheader()
        writer.writerow(row)


def ping_pair(source: str, target: str, path: str, source_ip: str, target_ip: str, output: Path) -> None:
    result = remote(
        source,
        ["env", "LC_ALL=C", "ping", "-I", source_ip, "-q", "-c", "10", "-i", "0.2", "-W", "3", target_ip],
        timeout=15,
    )
    text = result.stdout + result.stderr
    loss = re.search(r"([\d.]+)% packet loss", text)
    rtt = re.search(r"= [\d.]+/([\d.]+)/[\d.]+/[\d.]+ ms", text)
    write_row({
        "kind": "ping", "path": path, "source": source, "source_provider": NODES[source]["provider"],
        "target": target, "target_provider": NODES[target]["provider"], "direction": f"{source}->{target}",
        "repeat": 1, "protocol": "icmp", "offered_mbps": "", "throughput_mbps": "",
        "loss_pct": float(loss.group(1)) if loss else "100", "jitter_ms": "",
        "rtt_avg_ms": float(rtt.group(1)) if rtt else "", "retransmits": "",
        "seconds": 2, "error": "" if result.returncode == 0 else text.strip(),
    }, output)


def run_iperf(
    source: str, target: str, path: str, source_ip: str, target_ip: str,
    protocol: str, reverse: bool, repeat: int, seconds: int, offered_mbps: int | None,
    output: Path,
) -> None:
    server_pid = require_remote(
        target,
        ["sh", "-c", f"nohup iperf3 -s -B {target_ip} -p {PORT} -1 >/dev/null 2>&1 </dev/null & echo $!"],
    ).splitlines()[-1]
    try:
        # SSH crea una sesión nueva para el servidor; darle tiempo a iniciar
        # evita carreras entre la autenticación remota y el primer SYN.
        time.sleep(1.2)
        command = ["iperf3", "-c", target_ip, "-B", source_ip, "-p", str(PORT),
                   "-t", str(seconds), "-P", "1", "-J"]
        if reverse:
            command.append("-R")
        if protocol == "udp":
            command.extend(["-u", "-b", f"{offered_mbps}M", "-l", "1200"])

        result = remote(source, command, timeout=seconds + 25)
        if result.returncode and "unable to connect to server" in result.stderr:
            time.sleep(0.8)
            result = remote(source, command, timeout=seconds + 25)
        if result.returncode:
            raise RuntimeError(result.stderr.strip() or result.stdout.strip())
        data = json.loads(result.stdout)
        end = data.get("end", {})
        receiver = end.get("sum_received") or end.get("sum") or {}
        sender = end.get("sum_sent") or {}
        throughput = receiver.get("bits_per_second", 0) / 1_000_000
        write_row({
            "kind": "iperf3", "path": path, "source": source, "source_provider": NODES[source]["provider"],
            "target": target, "target_provider": NODES[target]["provider"],
            "direction": f"{target}->{source}" if reverse else f"{source}->{target}",
            "repeat": repeat, "protocol": protocol, "offered_mbps": offered_mbps or "",
            "throughput_mbps": round(throughput, 3),
            "loss_pct": round(receiver.get("lost_percent", 0), 3) if protocol == "udp" else "",
            "jitter_ms": round(receiver.get("jitter_ms", 0), 3) if protocol == "udp" else "",
            "rtt_avg_ms": "", "retransmits": sender.get("retransmits", ""),
            "seconds": seconds, "error": "",
        }, output)
    finally:
        # El listener se lanzó desacoplado de SSH; detenerlo por PID evita
        # procesos huérfanos incluso si iperf3 no logró aceptar una conexión.
        remote(target, ["kill", server_pid])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repeat", type=int, default=2)
    parser.add_argument("--tcp-seconds", type=int, default=6)
    parser.add_argument("--udp-seconds", type=int, default=4)
    parser.add_argument("--udp-rates", default="5,20", help="caudales UDP ofrecidos en Mbps")
    parser.add_argument("--pairs", help="lista separada por comas, por ejemplo fenix:kingu")
    parser.add_argument("--paths", default="wireguard,bat0", help="rutas a medir: wireguard,bat0")
    parser.add_argument("--protocols", default="tcp,udp", help="protocolos a medir: tcp,udp")
    parser.add_argument("--direction", choices=("both", "forward", "reverse"), default="both")
    parser.add_argument("--skip-ping", action="store_true", help="omite pings previos")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    pairs = PAIRS
    if args.pairs:
        requested = [tuple(item.split(":", 1)) for item in args.pairs.split(",")]
        if any(len(pair) != 2 or pair not in PAIRS for pair in requested):
            parser.error("--pairs debe seleccionar parejas de la matriz predeterminada")
        pairs = requested
    paths = [value for value in args.paths.split(",") if value]
    protocols = [value for value in args.protocols.split(",") if value]
    if not paths or any(value not in ("wireguard", "bat0") for value in paths):
        parser.error("--paths solo admite wireguard y bat0")
    if not protocols or any(value not in ("tcp", "udp") for value in protocols):
        parser.error("--protocols solo admite tcp y udp")
    directions = (False, True) if args.direction == "both" else (args.direction == "reverse",)
    rates = [int(value) for value in args.udp_rates.split(",") if value]
    timestamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    output = args.output or Path("docs/benchmarks/results") / f"red_{timestamp}.csv"
    global PORT, RUN_MARKER
    PORT = 52000 + (int(time.time()) % 10000)
    RUN_MARKER = f"mycnetbench{int(time.time())}"

    print(f"Puerto temporal: {PORT}; resultados: {output}")
    print("Abriendo reglas privadas de prueba; se retirarán al finalizar.")
    try:
        open_test_port()
        check_dns()
        for left, right in pairs:
            print(f"Pareja: {left} ({NODES[left]['provider']}) ↔ {right} ({NODES[right]['provider']})")
            for path in paths:
                left_ip = NODES[left]["wg" if path == "wireguard" else "bat"]
                right_ip = NODES[right]["wg" if path == "wireguard" else "bat"]
                if not args.skip_ping:
                    ping_pair(left, right, path, left_ip, right_ip, output)
                    ping_pair(right, left, path, right_ip, left_ip, output)
                for repeat in range(1, args.repeat + 1):
                    if "tcp" in protocols:
                        for reverse in directions:
                            run_iperf(left, right, path, left_ip, right_ip, "tcp", reverse,
                                      repeat, args.tcp_seconds, None, output)
                    if "udp" in protocols:
                        for rate in rates:
                            for reverse in directions:
                                run_iperf(left, right, path, left_ip, right_ip, "udp", reverse,
                                          repeat, args.udp_seconds, rate, output)
                check_dns()
        print(f"Mediciones guardadas en {output} ({len(ROWS)} filas).")
        return 0
    except Exception as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1
    finally:
        close_test_port()
        print("Reglas temporales de medición retiradas.")


if __name__ == "__main__":
    raise SystemExit(main())
