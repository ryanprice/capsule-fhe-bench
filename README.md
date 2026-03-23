# Capsule Protocol — FHE Benchmark Suite for NVIDIA DGX Spark (GB10)

## Purpose

This benchmark suite measures the real-world performance of TFHE (Fully Homomorphic Encryption) operations on the NVIDIA DGX Spark / GB10 hardware. The results directly inform the Capsule Protocol's computation class definitions, pricing model, and hardware requirements.

We test **exactly the operations a Capsule Node daemon would perform** when an AI agent pays to compute over encrypted personal data.

## Key Results

Benchmarks completed March 23, 2026 on DGX Spark (GB10 Grace Blackwell Superchip). Full report: [`results/BENCHMARK_REPORT.md`](results/BENCHMARK_REPORT.md).

| Computation Class | Target | CPU | GPU | Verdict |
|:--|:--|:--|:--|:--|
| **Class A** (simple queries) | < 1 second | 2,175 ms | **962 ms** | **Viable on GPU** |
| **Class B** (analytical) | < 30 seconds | 8,885 ms | **5,319 ms** | **Viable on both** |

Simple queries like glucose threshold checks complete in **50 ms on CPU** and **78 ms on GPU** — real-time on either backend. The GPU provides 1.6-2.5x acceleration on multi-step analytical queries. Memory usage is 0.26 GB peak.

See the [full benchmark report](results/BENCHMARK_REPORT.md) for per-operation breakdowns, scaling analysis, and recommendations.

## What We Measure

### Tier 1 — Primitive Operations
Raw FHE operation latency: addition, multiplication, comparison, division on encrypted integers of various bit widths (8, 16, 32, 64 bit). Establishes the baseline cost of each homomorphic operation on the GB10.

### Tier 2 — Class A Capsule Queries (Simple)
Pre-aggregated statistical queries that map to real capsule use cases:
- "Is encrypted glucose above threshold?" (50 ms CPU / 78 ms GPU)
- "Is encrypted credit score within range [680, 750]?" (116 ms CPU / 172 ms GPU)
- "How many of 10 encrypted features exceed their thresholds?" (1,844 ms CPU / 2,103 ms GPU)

### Tier 3 — Class B Capsule Queries (Analytical)
Multi-step computations over encrypted feature vectors:
- Dot product of two 10-element encrypted vectors (6,052 ms CPU / 5,306 ms GPU)
- Weighted sum across 20 encrypted features (4,935 ms CPU / 2,182 ms GPU)
- Min/max across 50 encrypted values (22,180 ms CPU / 12,852 ms GPU)

### Tier 4 — Key Generation & Setup
One-time costs: key generation (0.47s CPU / 0.78s GPU), encryption of a dataset (40 ms for 100 values), memory footprint (0.14-0.26 GB).

## Prerequisites

### On the DGX Spark:
```bash
# Verify CUDA is available
nvidia-smi
nvcc --version

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify Rust version (need 1.91.1+)
rustc --version
```

## Quick Start

```bash
cd capsule-fhe-bench

# Step 1: Run the hardware detection script
bash scripts/detect_hardware.sh

# Step 2: Build in release mode (first build takes 10-20 minutes)
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Step 3: Quick sanity check (3 iterations)
./target/release/capsule-bench --quick

# Step 4: Full CPU benchmark (10 iterations)
./target/release/capsule-bench --iterations 10 --output results/full_cpu_bench.json

# Step 5: Full GPU benchmark (10 iterations)
./target/release/capsule-bench-gpu --iterations 10 --output results/full_gpu_bench.json
```

Or run everything at once:
```bash
bash scripts/run_full_suite.sh
```

## GPU Benchmarks

GPU support is enabled by default in `Cargo.toml` using the `gpu` feature. The GPU binary uses the TFHE-rs integer GPU API with `CudaServerKey` and `CudaUnsignedRadixCiphertext`.

The GB10's Blackwell GPU (sm_12.1) is confirmed compatible with the TFHE-rs CUDA backend (tfhe-cuda-backend v0.13.2).

```bash
# Build GPU binary
RUSTFLAGS="-C target-cpu=native" cargo build --release --bin capsule-bench-gpu

# Quick GPU test
./target/release/capsule-bench-gpu --quick

# Full GPU run
./target/release/capsule-bench-gpu --iterations 10 --output results/full_gpu_bench.json
```

## Output

Results are written as JSON to `results/` with the following structure:
```json
{
  "hardware": { "device": "NVIDIA DGX Spark (GB10)", "cpu": "ARM CPU", "gpu": "NVIDIA GB10", "memory_gb": 121 },
  "timestamp": "2026-03-23T...",
  "tfhe_version": "1.5.x",
  "benchmarks": [
    {
      "name": "glucose_threshold_check",
      "category": "class_a",
      "capsule_class": "A",
      "description": "Single comparison: is encrypted mean glucose >= 140?",
      "bit_width": 16,
      "iterations": 10,
      "mean_ms": 49.51,
      "min_ms": 38.73,
      "max_ms": 71.06,
      "std_dev_ms": 11.98,
      "backend": "cpu"
    }
  ],
  "summary": {
    "total_benchmarks": 31,
    "class_a_viable": false,
    "class_a_mean_ms": 2174.56,
    "class_b_viable": true,
    "class_b_mean_ms": 8885.17,
    "key_gen_seconds": 0.47,
    "memory_estimate_gb": 0.14
  }
}
```

## File Structure

```
capsule-fhe-bench/
├── Cargo.toml                  # Rust project config (tfhe with integer + gpu features)
├── CLAUDE.md                   # Instructions for Claude Code
├── README.md                   # This file
├── src/
│   ├── main.rs                 # CPU benchmark binary
│   ├── main_gpu.rs             # GPU benchmark binary
│   ├── bench_primitives.rs     # Tier 1: raw FHE operations (8/16/32/64-bit)
│   ├── bench_class_a.rs        # Tier 2: simple capsule queries
│   ├── bench_class_b.rs        # Tier 3: analytical capsule queries
│   ├── bench_setup.rs          # Tier 4: key gen, encryption costs
│   └── reporting.rs            # Timing collection and JSON output formatting
├── scripts/
│   ├── detect_hardware.sh      # Hardware detection pre-flight check
│   └── run_full_suite.sh       # Automated full benchmark run
└── results/
    ├── BENCHMARK_REPORT.md     # Full benchmark report with analysis
    ├── full_cpu_bench.json     # CPU benchmark results (10 iterations)
    └── full_gpu_bench.json     # GPU benchmark results (10 iterations)
```
