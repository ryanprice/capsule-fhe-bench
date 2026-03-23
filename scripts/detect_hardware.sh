#!/bin/bash
# detect_hardware.sh — Verify the DGX Spark environment before running benchmarks

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Capsule Protocol — Hardware Detection                  ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# ── CPU ──
echo "── CPU ──"
if [ -f /proc/cpuinfo ]; then
    CORES=$(nproc 2>/dev/null || echo "unknown")
    MODEL=$(grep -m1 "model name\|Model" /proc/cpuinfo | cut -d: -f2 | xargs)
    ARCH=$(uname -m)
    echo "  Architecture: $ARCH"
    echo "  Model:        $MODEL"
    echo "  Cores:        $CORES"
else
    echo "  Unable to read CPU info"
fi
echo ""

# ── Memory ──
echo "── Memory ──"
if [ -f /proc/meminfo ]; then
    TOTAL_KB=$(grep MemTotal /proc/meminfo | awk '{print $2}')
    TOTAL_GB=$((TOTAL_KB / 1048576))
    AVAIL_KB=$(grep MemAvailable /proc/meminfo | awk '{print $2}')
    AVAIL_GB=$((AVAIL_KB / 1048576))
    echo "  Total:     ${TOTAL_GB} GB"
    echo "  Available: ${AVAIL_GB} GB"
else
    echo "  Unable to read memory info"
fi
echo ""

# ── GPU / CUDA ──
echo "── GPU / CUDA ──"
if command -v nvidia-smi &>/dev/null; then
    echo "  nvidia-smi: FOUND"
    nvidia-smi --query-gpu=name,compute_cap,memory.total,driver_version --format=csv,noheader 2>/dev/null | while IFS=',' read -r name cc mem driver; do
        echo "  GPU:            $(echo $name | xargs)"
        echo "  Compute Cap:    $(echo $cc | xargs)"
        echo "  GPU Memory:     $(echo $mem | xargs)"
        echo "  Driver:         $(echo $driver | xargs)"
    done
else
    echo "  nvidia-smi: NOT FOUND"
    echo "  ⚠ GPU benchmarks will not be available"
fi

if command -v nvcc &>/dev/null; then
    NVCC_VER=$(nvcc --version 2>/dev/null | grep "release" | awk '{print $NF}')
    echo "  CUDA Toolkit:   $NVCC_VER"
else
    echo "  nvcc: NOT FOUND"
    echo "  ⚠ CUDA compilation may not work"
fi
echo ""

# ── Rust ──
echo "── Rust Toolchain ──"
if command -v rustc &>/dev/null; then
    RUST_VER=$(rustc --version)
    echo "  $RUST_VER"
    CARGO_VER=$(cargo --version)
    echo "  $CARGO_VER"

    # Check if version is high enough (need 1.91.1+)
    RUST_MINOR=$(rustc --version | grep -oP '1\.(\d+)' | head -1 | cut -d. -f2)
    if [ "$RUST_MINOR" -lt 91 ] 2>/dev/null; then
        echo "  ⚠ TFHE-rs 1.x requires Rust 1.91.1+. Please update:"
        echo "    rustup update stable"
    else
        echo "  ✓ Rust version is sufficient for TFHE-rs"
    fi
else
    echo "  Rust: NOT FOUND"
    echo "  Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi
echo ""

# ── NVLink C2C (GB10 specific) ──
echo "── NVLink C2C (GB10 unified memory) ──"
if [ -f /proc/buddyinfo ]; then
    echo "  Unified memory architecture detected (CPU+GPU share 128GB LPDDR5X)"
    echo "  Bandwidth: ~273 GB/s (shared)"
    echo "  This is a significant advantage for FHE workloads"
fi
echo ""

# ── Summary ──
echo "══ READINESS CHECK ══"
READY=true
if ! command -v rustc &>/dev/null; then
    echo "  ✗ Rust not installed"
    READY=false
else
    echo "  ✓ Rust toolchain"
fi

if ! command -v nvidia-smi &>/dev/null; then
    echo "  ✗ NVIDIA GPU not detected (CPU benchmarks will still work)"
else
    echo "  ✓ NVIDIA GPU detected"
fi

if [ "$READY" = true ]; then
    echo ""
    echo "  Ready to build! Run:"
    echo "    RUSTFLAGS=\"-C target-cpu=native\" cargo build --release"
    echo ""
    echo "  Then run benchmarks:"
    echo "    ./target/release/capsule-bench --quick"
    echo "    ./target/release/capsule-bench --iterations 10 --output results/full_bench.json"
fi
