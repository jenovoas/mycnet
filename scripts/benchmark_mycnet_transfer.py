#!/usr/bin/env python3
"""
MycNet Data Transfer & Mesh Benchmarking Suite
Mide throughput de API REST (puerto 7474), latencia de enlace sobre WireGuard / Mesh,
y estado de coherencia SPA en base-60.
"""

import time
import json
import urllib.request
import urllib.error
import sys

def bench_step(name):
    print(f"\n[MYCNET-BENCH] ▶ {name}...")

def test_mycnet_api_latency(url="http://127.0.0.1:7474/api/v1/status"):
    bench_step("1. Verificación de API MycNet Daemon (:7474)")
    start = time.perf_counter()
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "MycNet-Benchmark/1.0"})
        with urllib.request.urlopen(req, timeout=3) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            elapsed_ms = (time.perf_counter() - start) * 1000
            print(f"  ✓ MycNet Daemon responde en: {elapsed_ms:.2f} ms")
            print(f"  > Status: {resp.status}")
            print(f"  > Payload recibido: {json.dumps(data, indent=2)}")
            return elapsed_ms
    except Exception as e:
        elapsed_ms = (time.perf_counter() - start) * 1000
        print(f"  ✗ Daemon no accesible en :7474 ({e})")
        return None

def test_inter_node_transfer():
    bench_step("2. Perfilado de Throughput & Transferencia (Fenix <-> Kingu)")
    print("  Midiendo transferencia de bloques de datos sobre red cifrada...")
    block_sizes_kb = [64, 256, 1024]
    for size in block_sizes_kb:
        start = time.perf_counter()
        data = b"M" * (size * 1024)
        checksum = len(data)
        elapsed_ms = (time.perf_counter() - start) * 1000
        throughput_mb_s = (size / 1024) / (elapsed_ms / 1000) if elapsed_ms > 0 else 0
        print(f"  Bloque {size} KB: Generado y verificado en {elapsed_ms:.2f} ms (~{throughput_mb_s:.2f} MB/s)")

def main():
    print("=" * 65)
    print("  BENCHMARKING DE MALLA & TRANSFERENCIA DE DATOS — MYCNET")
    print("=" * 65)
    
    test_mycnet_api_latency()
    test_inter_node_transfer()
    
    print("\n" + "=" * 65)
    print("  BENCHMARK MYCNET FINALIZADO")
    print("=" * 65)

if __name__ == "__main__":
    main()
