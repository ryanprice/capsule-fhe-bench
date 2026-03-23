# CLAUDE.md — Instructions for Claude Code

## Context

This is the **Capsule Protocol FHE Benchmark Suite**. We are benchmarking Fully Homomorphic Encryption (TFHE) operations on an NVIDIA DGX Spark (GB10 Grace Blackwell Superchip) to determine the real-world feasibility of running encrypted computations on consumer/prosumer hardware for a self-sovereign data protocol.

The DGX Spark specs:
- **CPU**: 20-core ARM (10x Cortex-X925 + 10x Cortex-A725)
- **GPU**: Blackwell GPU, 6144 CUDA cores, CUDA compute capability sm_12x
- **Memory**: 128GB unified LPDDR5X (shared CPU+GPU via NVLink C2C, 273 GB/s)
- **OS**: DGX OS (Ubuntu 24.04 LTS based)
- **Architecture**: aarch64 (ARM)

## Goal

Run TFHE-rs benchmarks mapped to the Capsule Protocol's computation classes:
- **Class A (simple queries)**: Must complete in <1 second to be viable for real-time agent queries
- **Class B (analytical)**: Must complete in <30 seconds to be viable for research/bounty workloads
- **Class C (complex)**: Informational — establish what's currently out of reach

The results will be published as proof that the protocol works on real hardware.

## Build Instructions

```bash
# First run: detect hardware
bash scripts/detect_hardware.sh

# Build (IMPORTANT: always use target-cpu=native for ARM optimization)
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Quick test (3 iterations per benchmark)
./target/release/capsule-bench --quick

# Full run (10 iterations)
./target/release/capsule-bench --iterations 10 --output results/full_bench.json
```

## Likely Issues & How to Fix Them

### Issue: TFHE-rs version mismatch
The tfhe crate version in Cargo.toml may need adjustment. Check https://crates.io/crates/tfhe for the latest version. As of March 2026, v1.5.x should be current. If compilation fails on version, try:
```bash
cargo search tfhe
# Then update the version in Cargo.toml
```

### Issue: Rust version too old
TFHE-rs 1.x requires Rust 1.91.1+. Fix with:
```bash
rustup update stable
# or
rustup install nightly && rustup default nightly
```

### Issue: API changes in TFHE-rs
The TFHE-rs high-level API has been relatively stable since v1.0, but if method signatures have changed:
- Check docs at https://docs.zama.org/tfhe-rs
- `FheUint8/16/32/64::encrypt()`, `decrypt()`, and arithmetic ops are core and unlikely to break
- The `cast_into()` method in bench_class_b.rs converts between bit widths — if it doesn't exist, try `FheUint32::try_from(val)` or explicit casting APIs
- `if_then_else` for conditional selection might be `select` or `cmux` in different versions

### Issue: Compilation on ARM / aarch64
TFHE-rs supports ARM but the SIMD optimizations (AVX2/AVX512) obviously don't apply. The library should auto-detect and use NEON SIMD instead. If there are arch-specific feature flags failing:
```bash
# Try without arch-specific features
RUSTFLAGS="-C target-cpu=native" cargo build --release --no-default-features --features integer
```

### Issue: GPU features
Do NOT enable GPU features until CPU benchmarks are working. The GPU path is a separate step:
1. First get CPU benchmarks running and collect baseline numbers
2. Then uncomment the GPU feature in Cargo.toml
3. The CUDA kernels may need sm_12x support — if TFHE-rs CUDA backend doesn't support it yet, we document that finding

### Issue: Long compilation time
First build of TFHE-rs can take 10-20 minutes. This is normal — it's compiling cryptographic primitives with heavy optimization. Subsequent builds are incremental and fast.

### Issue: Out of memory during build
Unlikely with 128GB, but if it happens:
```bash
# Limit parallel compilation
CARGO_BUILD_JOBS=4 RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## After Benchmarks Run

1. Read the JSON output in results/ 
2. The summary section shows whether Class A and Class B queries are viable
3. Key numbers we care about:
   - `key_generation` time (one-time cost at node startup)
   - `glucose_threshold_check` latency (simplest real capsule query)
   - `multi_threshold_10_features` latency (realistic Class A workload)
   - `dot_product_10_elem` latency (Class B analytical)
   - `minmax_50_values` latency (Class B edge case)
   - Memory RSS after key gen (tells us if 128GB is overkill or necessary)

4. If Class A queries are sub-second on CPU alone, that's a landmark result — it means any GB10 owner can serve real-time capsule queries without GPU acceleration

5. Share the results JSON — it feeds directly into the protocol spec for hardware recommendations

## Project Structure

```
src/main.rs              — Entry point, runs all benchmarks, produces JSON report
src/main_gpu.rs          — GPU benchmark stub (enable after CPU works)
src/reporting.rs         — Timing collection and JSON output formatting
src/bench_setup.rs       — Key generation, encryption/decryption costs
src/bench_primitives.rs  — Raw FHE ops at 8/16/32/64 bit widths
src/bench_class_a.rs     — Simulated Class A capsule queries (glucose, credit, etc.)
src/bench_class_b.rs     — Simulated Class B analytical queries (dot product, weighted score)
scripts/detect_hardware.sh  — Pre-flight hardware check
scripts/run_full_suite.sh   — Automated full benchmark run
```
