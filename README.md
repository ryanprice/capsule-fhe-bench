# Capsule Protocol — FHE Benchmark Suite for NVIDIA DGX Spark (GB10)

## Purpose

This benchmark suite measures the real-world performance of TFHE (Fully Homomorphic Encryption) operations on the NVIDIA DGX Spark / GB10 hardware. The results directly inform the Capsule Protocol's computation class definitions, pricing model, and hardware requirements.

We are testing **exactly the operations a Capsule Node daemon would perform** when an AI agent pays to compute over encrypted personal data.

## What We're Measuring

### Tier 1 — Primitive Operations
Raw FHE operation latency: addition, multiplication, comparison, division on encrypted integers of various bit widths (8, 16, 32, 64 bit). This establishes the baseline cost of each homomorphic operation on the GB10.

### Tier 2 — Class A Capsule Queries (Simple)
Pre-aggregated statistical queries that map to real capsule use cases:
- "What is the mean of these 50 encrypted feature values?"
- "Is encrypted value X greater than threshold Y?"
- "How many of these 20 encrypted values exceed a threshold?"
These are the sub-second queries we expect to serve in real-time.

### Tier 3 — Class B Capsule Queries (Analytical)
Multi-step computations over encrypted feature vectors:
- Dot product of two encrypted vectors (for correlation)
- Weighted sum across encrypted features (for scoring)
- Min/max/range across encrypted aggregates
These should complete in seconds.

### Tier 4 — Key Generation & Setup
One-time costs: key generation, encryption of a dataset, bootstrapping key size in memory. These affect node startup time and memory requirements.

### Tier 5 — GPU vs CPU Comparison
Same operations on CPU-only (ARM Grace cores) vs GPU (Blackwell CUDA cores) to quantify the GPU acceleration factor on unified memory architecture.

## Prerequisites

### On the DGX Spark:
```bash
# Verify CUDA is available
nvidia-smi
nvcc --version

# Verify Rust is installed (or install it)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify Rust version (need 1.91.1+)
rustc --version

# Install nightly toolchain (recommended for best TFHE-rs perf)
rustup install nightly
rustup default nightly
```

## Quick Start

```bash
# Clone or copy this project to the Spark
cd capsule-fhe-bench

# Step 1: Run the hardware detection script
bash scripts/detect_hardware.sh

# Step 2: Build in release mode (this will take a few minutes first time)
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Step 3: Run CPU benchmarks
./target/release/capsule-bench --mode cpu --output results/cpu_bench.json

# Step 4: If CUDA is working, enable GPU features in Cargo.toml and run GPU benchmarks
# (see GPU Setup section below)
```

## GPU Setup

TFHE-rs CUDA support requires the CUDA toolkit. On DGX Spark this should be pre-installed.

1. Verify CUDA:
```bash
nvcc --version
ls /usr/local/cuda/lib64/libcudart.so
```

2. Edit `Cargo.toml` — change the tfhe dependency to enable GPU:
```toml
tfhe = { version = "1.5", features = ["integer", "boolean", "gpu"] }
```

3. Build and run:
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release --bin capsule-bench-gpu
./target/release/capsule-bench-gpu --output results/gpu_bench.json
```

## Output

Results are written as JSON to `results/` with the following structure:
```json
{
  "hardware": { "device": "DGX Spark GB10", "cpu": "...", "gpu": "...", "memory_gb": 128 },
  "timestamp": "2026-03-21T...",
  "benchmarks": [
    {
      "name": "add_64bit_encrypted_encrypted",
      "category": "primitive",
      "capsule_class": "A",
      "iterations": 100,
      "mean_ms": 12.3,
      "min_ms": 11.8,
      "max_ms": 14.1,
      "std_dev_ms": 0.7,
      "backend": "cpu"
    }
  ]
}
```

## File Structure

```
capsule-fhe-bench/
├── Cargo.toml                  # Rust project config
├── README.md                   # This file
├── src/
│   ├── main.rs                 # CPU benchmark binary
│   ├── main_gpu.rs             # GPU benchmark binary
│   ├── bench_primitives.rs     # Tier 1: raw FHE operations
│   ├── bench_class_a.rs        # Tier 2: simple capsule queries
│   ├── bench_class_b.rs        # Tier 3: analytical capsule queries
│   ├── bench_setup.rs          # Tier 4: key gen, encryption costs
│   ├── capsule_sim.rs          # Simulated capsule data (glucose, finance)
│   └── reporting.rs            # JSON output formatting
├── scripts/
│   ├── detect_hardware.sh      # Hardware detection script
│   ├── run_full_suite.sh       # Run everything and generate report
│   └── compare_results.py      # Compare CPU vs GPU results
└── results/                    # Output directory (created at runtime)
```
