#!/usr/bin/env python3
"""
Benchmark comparativo de TCP público, WireGuard, SSH y semillas MycNet reales.
El canal MycNet observa los contadores del daemon emisor y receptor; no genera semillas.
"""

import argparse
import json
import socket
import subprocess
import time
import urllib.request

PUBLIC_IP = "68.211.176.190"
WG_IP     = "10.88.0.2"
PORT      = 9444

def send_tcp_stream(host, port, size_mb):
    total_bytes = size_mb * 1024 * 1024
    chunk = b"X" * 65536
    chunks_to_send = total_bytes // len(chunk)
    
    start = time.perf_counter()
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(15.0)
            s.connect((host, port))
            for _ in range(chunks_to_send):
                s.sendall(chunk)
            s.shutdown(socket.SHUT_WR)
        elapsed = time.perf_counter() - start
        throughput = size_mb / elapsed if elapsed > 0 else 0
        return elapsed, throughput
    except Exception as e:
        return None, str(e)

def send_ssh_stream(size_mb):
    total_bytes = size_mb * 1024 * 1024
    chunk = b"S" * 65536
    chunks = total_bytes // len(chunk)
    
    start = time.perf_counter()
    try:
        proc = subprocess.Popen(
            ["ssh", "-p", "4222", "-o", "StrictHostKeyChecking=no", "kingu", "cat > /dev/null"],
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )
        for _ in range(chunks):
            proc.stdin.write(chunk)
        proc.stdin.close()
        proc.wait()
        elapsed = time.perf_counter() - start
        throughput = size_mb / elapsed if elapsed > 0 else 0
        return elapsed, throughput
    except Exception as e:
        return None, str(e)

def fetch_seed_health(url):
    request = urllib.request.Request(url, headers={"User-Agent": "MycNet-Benchmark/1.0"})
    with urllib.request.urlopen(request, timeout=3) as response:
        data = json.loads(response.read().decode("utf-8"))

    required_counters = (
        "fractal_packet_bytes",
        "fractal_packets_sent",
        "fractal_seeds_ingested",
    )
    if not isinstance(data, dict) or data.get("status") != "OK":
        raise ValueError("respuesta de salud inválida")
    if not isinstance(data.get("fractal_source_connected"), bool):
        raise ValueError("falta fractal_source_connected")
    for name in required_counters:
        value = data.get(name)
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            raise ValueError(f"contador inválido: {name}")
        if name == "fractal_packet_bytes" and value == 0:
            raise ValueError("tamaño de paquete inválido")
    return data


def benchmark_mycnet_seeds(source_url, receiver_url, window_seconds):
    print("\n▶ Semillas MycNet reales (SHM → UDP → ingestión)...")
    try:
        source_before = fetch_seed_health(source_url)
        receiver_before = fetch_seed_health(receiver_url)
        if not source_before["fractal_source_connected"]:
            print("  Sin medición: el daemon emisor no tiene conectado el ring SHM de semillas.")
            return
        packet_bytes = source_before["fractal_packet_bytes"]
        if receiver_before["fractal_packet_bytes"] != packet_bytes:
            print("  Sin medición: los daemons reportan tamaños de paquete distintos.")
            return

        start_ns = time.monotonic_ns()
        time.sleep(window_seconds)
        source_after = fetch_seed_health(source_url)
        receiver_after = fetch_seed_health(receiver_url)
        elapsed_ns = time.monotonic_ns() - start_ns
    except Exception as error:
        print(f"  Sin medición: no se pudo consultar ambos daemons: {error}")
        return

    if not source_after["fractal_source_connected"]:
        print("  Sin medición: se perdió la conexión del emisor con el ring SHM.")
        return

    if (
        source_after["fractal_packet_bytes"] != packet_bytes
        or receiver_after["fractal_packet_bytes"] != packet_bytes
    ):
        print("  Sin medición: cambió el tamaño del paquete durante la ventana.")
        return

    sent_before = source_before["fractal_packets_sent"]
    sent_after = source_after["fractal_packets_sent"]
    received_before = receiver_before["fractal_seeds_ingested"]
    received_after = receiver_after["fractal_seeds_ingested"]
    if sent_after < sent_before or received_after < received_before:
        print("  Sin medición: un contador se reinició durante la ventana.")
        return

    sent = sent_after - sent_before
    received = received_after - received_before
    if sent == 0:
        print("  Sin medición: no se transmitieron semillas reales desde SHM durante la ventana.")
        return

    print(f"  Paquetes UDP enviados por el emisor: {sent} ({sent * packet_bytes} bytes de paquete)")
    print(f"  Semillas recibidas e inyectadas en destino: {received} ({received * packet_bytes} bytes de paquete)")
    print("  Nota: el contador de destino es global; no correlaciona cada paquete con este emisor.")
    if received == 0:
        print("  Recepción extremo a extremo no confirmada.")
    elif elapsed_ns > 0:
        rate_milli = received * 1_000_000_000_000 // elapsed_ns
        print(f"  Tasa de ingestión observada: {rate_milli // 1000}.{rate_milli % 1000:03d} semillas/s")

def main():
    parser = argparse.ArgumentParser(description="Benchmark de transporte MycNet")
    parser.add_argument("--mycnet-source-health", help="URL /health del daemon emisor")
    parser.add_argument("--mycnet-receiver-health", help="URL /health del daemon receptor")
    parser.add_argument("--mycnet-window-seconds", type=int, help="Duración entera de observación")
    args = parser.parse_args()

    mycnet_options = (
        args.mycnet_source_health,
        args.mycnet_receiver_health,
        args.mycnet_window_seconds,
    )
    if any(value is not None for value in mycnet_options) and not all(
        value is not None for value in mycnet_options
    ):
        parser.error("los tres parámetros --mycnet-* son necesarios para medir semillas reales")
    if args.mycnet_window_seconds is not None and args.mycnet_window_seconds <= 0:
        parser.error("--mycnet-window-seconds debe ser mayor que cero")

    print("=" * 75)
    print("  BENCHMARK COMPARATIVO MULTI-TRANSPORTE: FENIX <-> KINGU")
    print("=" * 75)
    print(f"  Nodo Origen:  Fenix (Azure A - 20.226.112.222 / 10.88.0.1)")
    print(f"  Nodo Destino: Kingu (Azure B - 68.211.176.190 / 10.88.0.2)")
    print("-" * 75)
    
    sizes = [10, 50]  # 10 MB y 50 MB
    
    for size in sizes:
        print(f"\n▶ PROBANDO VOLUMEN MASIVO: {size} MB ({size * 1024} KB)")
        print("-" * 75)
        
        # 1. TCP Directo por Internet Pública
        print(f"  [1] TCP Directo Internet ({PUBLIC_IP}:{PORT})...", end="", flush=True)
        t_tcp, mbps_tcp = send_tcp_stream(PUBLIC_IP, PORT, size)
        if t_tcp:
            print(f" ✓ {t_tcp:.3f} s  | Throughput: {mbps_tcp:.2f} MB/s (~{mbps_tcp*8:.2f} Mbps)")
        else:
            print(f" ✗ Falló: {mbps_tcp}")
            
        # 2. WireGuard VPN (wg0)
        print(f"  [2] WireGuard VPN ({WG_IP}:{PORT})...", end="", flush=True)
        t_wg, mbps_wg = send_tcp_stream(WG_IP, PORT, size)
        if t_wg:
            print(f"       ✓ {t_wg:.3f} s  | Throughput: {mbps_wg:.2f} MB/s (~{mbps_wg*8:.2f} Mbps)")
        else:
            print(f" ✗ Falló: {mbps_wg}")
            
        # 3. Túnel SSH Cifrado (Port 4222)
        print(f"  [3] Túnel Cifrado SSH (:4222)...", end="", flush=True)
        t_ssh, mbps_ssh = send_ssh_stream(size)
        if t_ssh:
            print(f"            ✓ {t_ssh:.3f} s  | Throughput: {mbps_ssh:.2f} MB/s (~{mbps_ssh*8:.2f} Mbps)")
        else:
            print(f" ✗ Falló: {mbps_ssh}")

    if all(value is not None for value in mycnet_options):
        benchmark_mycnet_seeds(*mycnet_options)
    else:
        print("\n▶ Semillas MycNet reales...")
        print("  Sin medición: configura las URL de ambos daemons y la ventana de observación.")

    print("\n" + "=" * 75)
    print("  BENCHMARK FINALIZADO; revisa el resultado de cada canal")
    print("=" * 75)

if __name__ == '__main__':
    main()
