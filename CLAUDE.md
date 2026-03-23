# CLAUDE.md — Instructions for Claude Code

## Context

This is the **Capsule Protocol FHE Benchmark Suite**. We benchmark Fully Homomorphic Encryption (TFHE) operations on an NVIDIA DGX Spark (GB10 Grace Blackwell Superchip) to determine the real-world feasibility of running encrypted computations on consumer/prosumer hardware for a self-sovereign data protocol.

The DGX Spark specs:
- **CPU**: 20-core ARM (10x Cortex-X925 + 10x Cortex-A725)
- **GPU**: Blackwell GPU, 6144 CUDA cores, CUDA compute capability sm_12.1
- **Memory**: 128GB unified LPDDR5X (shared CPU+GPU via NVLink C2C, 273 GB/s)
- **OS**: DGX OS (Ubuntu 24.04 LTS based), Linux 6.17.0
- **Architecture**: aarch64 (ARM)
- **CUDA**: Toolkit 13.0, Driver 580.126.09

## Current Status

**CPU and GPU benchmarks are complete.** Full results are in `results/`. Key findings:

- **Class A (simple queries):** Viable on GPU (962 ms mean). Simple threshold checks are 50-172 ms. Multi-step queries (sum 20 values, 10-feature scan) exceed 1 second.
- **Class B (analytical):** Viable on both backends. GPU provides 1.7x mean speedup. Heaviest workload (min/max 50 values) is 12.9s on GPU.
- **GPU acceleration:** 1.6-2.5x speedup on multi-step operations. CPU is faster for trivial 1-2 operation queries due to GPU kernel launch overhead.
- **Memory:** 0.26 GB peak RSS. The 128 GB unified memory is not a constraint.
- **Recommendation:** Hybrid CPU/GPU routing — CPU for simple threshold queries, GPU for analytical workloads.

Full report: `results/BENCHMARK_REPORT.md`

## Build Instructions

```bash
# First run: detect hardware
bash scripts/detect_hardware.sh

# Build both CPU and GPU binaries
# IMPORTANT: always use target-cpu=native for ARM optimization
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Quick test — CPU (3 iterations per benchmark)
./target/release/capsule-bench --quick

# Quick test — GPU (3 iterations per benchmark)
./target/release/capsule-bench-gpu --quick

# Full run — CPU (10 iterations)
./target/release/capsule-bench --iterations 10 --output results/full_cpu_bench.json

# Full run — GPU (10 iterations)
./target/release/capsule-bench-gpu --iterations 10 --output results/full_gpu_bench.json
```

## Known Issues & Fixes

### Issue: TFHE-rs version mismatch
Currently using tfhe 1.5.4. Check https://crates.io/crates/tfhe for updates. If compilation fails:
```bash
cargo search tfhe
# Then update the version in Cargo.toml
```

### Issue: Rust version too old
TFHE-rs 1.x requires Rust 1.91.1+. Currently using Rust 1.94.0 stable. Fix with:
```bash
rustup update stable
```

### Issue: API changes in TFHE-rs
- **CPU (high-level API):** `FheUint8/16/32/64::encrypt()`, `decrypt()`, arithmetic ops, `.ge()`, `.if_then_else()` are core and stable.
- **CPU casting:** `cast_into()` requires owned values, not references. Use `.clone().cast_into()` when iterating.
- **GPU (integer API):** Uses `CudaServerKey`, `CudaUnsignedRadixCiphertext`, `gen_keys_radix_gpu()`. NOT the high-level API.
- **GPU trivial creation:** `create_trivial_radix` needs turbofish: `server_key.create_trivial_radix::<_, CudaUnsignedRadixCiphertext>(0u64, num_blocks, &streams)`
- **GPU boolean ops:** Use `boolean_bitand()` for AND on `CudaBooleanBlock` (not `bitand()`, which requires `CudaIntegerRadixCiphertext`).
- **GPU parameters:** `PARAM_GPU_MULTI_BIT_GROUP_4_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128`

### Issue: Compilation on ARM / aarch64
TFHE-rs auto-detects and uses NEON SIMD on ARM. If arch-specific features fail:
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release --no-default-features --features integer
```

### Issue: GPU CUDA backend
The GB10's sm_12.1 (Blackwell) is confirmed compatible with tfhe-cuda-backend v0.13.2. GPU features are enabled by default in Cargo.toml (`features = ["integer", "gpu"]`).

### Issue: Long compilation time
First build takes 10-20 minutes (TFHE-rs cryptographic primitives). Subsequent builds are incremental (~3-5 seconds).

### Issue: Out of memory during build
Unlikely with 128GB, but if it happens:
```bash
CARGO_BUILD_JOBS=4 RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Key Benchmark Results

| Metric | CPU | GPU |
|:--|--:|--:|
| Key generation | 467 ms | 784 ms |
| Glucose threshold (Class A) | 50 ms | 78 ms |
| Credit range check (Class A) | 116 ms | 172 ms |
| Sum 20 values (Class A) | 3,141 ms | 1,495 ms |
| Weighted score 20 feat (Class B) | 4,935 ms | 2,182 ms |
| Min/Max 50 values (Class B) | 22,180 ms | 12,852 ms |
| Rolling sum 12 months (Class B) | 2,373 ms | 935 ms |
| Memory RSS | 0.14 GB | 0.26 GB |

## Project Structure

```
src/main.rs              — CPU benchmark entry point, runs all tiers, produces JSON report
src/main_gpu.rs          — GPU benchmark entry point, uses TFHE-rs integer GPU API
src/reporting.rs         — Timing collection and JSON output formatting (CPU only)
src/bench_setup.rs       — Key generation, encryption/decryption costs (CPU)
src/bench_primitives.rs  — Raw FHE ops at 8/16/32/64 bit widths (CPU)
src/bench_class_a.rs     — Simulated Class A capsule queries (CPU)
src/bench_class_b.rs     — Simulated Class B analytical queries (CPU)
scripts/detect_hardware.sh  — Pre-flight hardware check
scripts/run_full_suite.sh   — Automated full benchmark run
results/BENCHMARK_REPORT.md — Full benchmark report with analysis
results/full_cpu_bench.json — CPU results (10 iterations, 31 benchmarks)
results/full_gpu_bench.json — GPU results (10 iterations, 15 benchmarks)
```
