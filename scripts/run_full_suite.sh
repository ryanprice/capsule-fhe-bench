#!/bin/bash
# run_full_suite.sh — Run the complete Capsule Protocol FHE benchmark suite
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$PROJECT_DIR/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

mkdir -p "$RESULTS_DIR"

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Capsule Protocol — Full FHE Benchmark Suite            ║"
echo "║  $(date)                              ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Step 1: Hardware detection
echo "Step 1: Detecting hardware..."
bash "$SCRIPT_DIR/detect_hardware.sh" | tee "$RESULTS_DIR/hardware_${TIMESTAMP}.txt"
echo ""

# Step 2: Build
echo "Step 2: Building in release mode (this may take several minutes on first run)..."
cd "$PROJECT_DIR"
RUSTFLAGS="-C target-cpu=native" cargo build --release 2>&1 | tail -5
echo "  ✓ Build complete"
echo ""

# Step 3: Quick benchmark (3 iterations, fast sanity check)
echo "Step 3: Running quick benchmark (3 iterations)..."
./target/release/capsule-bench \
    --quick \
    --output "$RESULTS_DIR/quick_${TIMESTAMP}.json"
echo ""

# Step 4: Full benchmark (10 iterations)
echo "Step 4: Running full benchmark (10 iterations)..."
./target/release/capsule-bench \
    --iterations 10 \
    --output "$RESULTS_DIR/full_${TIMESTAMP}.json"
echo ""

# Step 5: Print results location
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  COMPLETE                                               ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Results saved to:                                      ║"
echo "║    $RESULTS_DIR/quick_${TIMESTAMP}.json"
echo "║    $RESULTS_DIR/full_${TIMESTAMP}.json"
echo "║    $RESULTS_DIR/hardware_${TIMESTAMP}.txt"
echo "╚══════════════════════════════════════════════════════════╝"
