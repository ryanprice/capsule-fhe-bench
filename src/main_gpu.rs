// GPU benchmark entry point for NVIDIA GB10
//
// Mirrors the CPU benchmarks in main.rs but runs all FHE operations on the GPU.
// Uses TFHE-rs integer GPU API: gen_keys_radix_gpu, CudaServerKey, CudaUnsignedRadixCiphertext.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use tfhe::core_crypto::gpu::CudaStreams;
use tfhe::core_crypto::gpu::vec::GpuIndex;
use tfhe::integer::gpu::ciphertext::CudaUnsignedRadixCiphertext;
use tfhe::integer::gpu::gen_keys_radix_gpu;
use tfhe::shortint::parameters::PARAM_GPU_MULTI_BIT_GROUP_4_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128;

// ── Reporting types (standalone, no module dependency) ──

#[derive(Serialize, Deserialize, Clone)]
struct HardwareInfo {
    device: String,
    cpu: String,
    gpu: String,
    memory_gb: u64,
    os: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct BenchmarkResult {
    name: String,
    category: String,
    capsule_class: String,
    description: String,
    bit_width: u32,
    iterations: u32,
    mean_ms: f64,
    min_ms: f64,
    max_ms: f64,
    std_dev_ms: f64,
    backend: String,
}

#[derive(Serialize, Deserialize)]
struct BenchmarkReport {
    hardware: HardwareInfo,
    timestamp: String,
    tfhe_version: String,
    benchmarks: Vec<BenchmarkResult>,
    summary: ReportSummary,
}

#[derive(Serialize, Deserialize)]
struct ReportSummary {
    total_benchmarks: usize,
    class_a_viable: bool,
    class_a_mean_ms: f64,
    class_b_viable: bool,
    class_b_mean_ms: f64,
    key_gen_seconds: f64,
    memory_estimate_gb: f64,
}

// ── Helpers ──

fn run_timed<F>(name: &str, iterations: u32, mut f: F) -> Vec<Duration>
where
    F: FnMut(),
{
    let mut timings = Vec::with_capacity(iterations as usize);
    println!("  [warmup] {}...", name);
    f();
    for i in 0..iterations {
        if iterations > 5 && i % 5 == 0 {
            println!("  [{}/{}] {}...", i + 1, iterations, name);
        }
        let start = Instant::now();
        f();
        timings.push(start.elapsed());
    }
    timings
}

fn summarize(
    name: &str, category: &str, capsule_class: &str, description: &str,
    bit_width: u32, timings: &[Duration],
) -> BenchmarkResult {
    let ms_values: Vec<f64> = timings.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    let n = ms_values.len() as f64;
    let mean = ms_values.iter().sum::<f64>() / n;
    let min = ms_values.iter().cloned().fold(f64::MAX, f64::min);
    let max = ms_values.iter().cloned().fold(f64::MIN, f64::max);
    let variance = ms_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();

    println!(
        "  ✓ {} — mean: {:.2}ms | min: {:.2}ms | max: {:.2}ms | stddev: {:.2}ms",
        name, mean, min, max, std_dev
    );

    BenchmarkResult {
        name: name.to_string(),
        category: category.to_string(),
        capsule_class: capsule_class.to_string(),
        description: description.to_string(),
        bit_width,
        iterations: timings.len() as u32,
        mean_ms: (mean * 100.0).round() / 100.0,
        min_ms: (min * 100.0).round() / 100.0,
        max_ms: (max * 100.0).round() / 100.0,
        std_dev_ms: (std_dev * 100.0).round() / 100.0,
        backend: "gpu".to_string(),
    }
}

fn detect_hardware() -> HardwareInfo {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("model name") || l.starts_with("Model"))
        .map(|l| l.split(':').nth(1).unwrap_or("unknown").trim().to_string())
        .unwrap_or_else(|| "ARM CPU".to_string());

    let gpu = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=name", "--format=csv,noheader"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "No GPU detected".to_string());

    let memory_gb = std::fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("MemTotal:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u64>().ok())
        .map(|kb| kb / 1_048_576)
        .unwrap_or(0);

    let os = std::process::Command::new("uname")
        .args(["-srm"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    HardwareInfo {
        device: "NVIDIA DGX Spark (GB10)".to_string(),
        cpu,
        gpu,
        memory_gb,
        os,
    }
}

// Number of 2-bit message blocks needed per bit width:
//   8-bit  = 4 blocks
//   16-bit = 8 blocks
//   32-bit = 16 blocks
//   64-bit = 32 blocks

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  CAPSULE PROTOCOL — FHE GPU BENCHMARK SUITE            ║");
    println!("║  Target: NVIDIA GB10 Blackwell GPU (6144 CUDA cores)   ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Parse args
    let args: Vec<String> = std::env::args().collect();
    let output_path = args.iter()
        .position(|a| a == "--output")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "results/gpu_bench_results.json".to_string());

    let iterations: u32 = args.iter()
        .position(|a| a == "--iterations")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let quick_mode = args.contains(&"--quick".to_string());
    let iters = if quick_mode { std::cmp::min(iterations, 3) } else { iterations };

    // Detect hardware
    println!("Detecting hardware...");
    let hardware = detect_hardware();
    println!("  Device: {}", hardware.device);
    println!("  CPU:    {}", hardware.cpu);
    println!("  GPU:    {}", hardware.gpu);
    println!("  Memory: {} GB", hardware.memory_gb);
    println!("  OS:     {}", hardware.os);
    println!("  Iterations per benchmark: {}", iters);
    if quick_mode {
        println!("  Mode: QUICK (reduced iterations)");
    }

    let mut all_results = Vec::new();

    // ══════════════════════════════════════════════════════════
    // Tier 4: Setup — GPU key generation
    // ══════════════════════════════════════════════════════════
    println!("\n══ Tier 4: GPU Setup & Initialization ══\n");

    println!("── GPU initialization ──");
    let gpu_init_start = Instant::now();
    let streams = CudaStreams::new_single_gpu(GpuIndex::new(0));
    let gpu_init_ms = gpu_init_start.elapsed().as_secs_f64() * 1000.0;
    println!("  ✓ GPU stream init: {:.2}ms", gpu_init_ms);

    // Use 16 blocks for 32-bit operations (2 bits per block × 16 = 32 bits)
    // This is our primary bit width for benchmarks
    let num_blocks_32 = 16;

    println!("\n── Key generation (GPU, 32-bit / {} blocks) ──", num_blocks_32);
    let keygen_start = Instant::now();
    let (client_key, server_key) = gen_keys_radix_gpu(
        PARAM_GPU_MULTI_BIT_GROUP_4_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128,
        num_blocks_32,
        &streams,
    );
    let keygen_ms = keygen_start.elapsed().as_secs_f64() * 1000.0;
    println!("  ✓ GPU key generation: {:.2}s", keygen_ms / 1000.0);

    all_results.push(BenchmarkResult {
        name: "key_generation".to_string(),
        category: "setup".to_string(),
        capsule_class: "setup".to_string(),
        description: "GPU key generation (radix, 32-bit)".to_string(),
        bit_width: 32,
        iterations: 1,
        mean_ms: (keygen_ms * 100.0).round() / 100.0,
        min_ms: (keygen_ms * 100.0).round() / 100.0,
        max_ms: (keygen_ms * 100.0).round() / 100.0,
        std_dev_ms: 0.0,
        backend: "gpu".to_string(),
    });

    // ══════════════════════════════════════════════════════════
    // Tier 1: Primitive 32-bit operations on GPU
    // ══════════════════════════════════════════════════════════
    println!("\n══ Tier 1: Primitive Operations (32-bit, GPU) ══\n");

    // Encrypt values on CPU, transfer to GPU
    let ct_a_cpu = client_key.encrypt(42u32);
    let ct_b_cpu = client_key.encrypt(17u32);
    let d_a = CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct_a_cpu, &streams);
    let d_b = CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct_b_cpu, &streams);

    // Addition
    let timings = run_timed("add_32bit_gpu", iters, || {
        let _result = server_key.add(&d_a, &d_b, &streams);
        streams.synchronize();
    });
    all_results.push(summarize(
        "add_32bit_gpu", "primitive", "A",
        "Addition of two encrypted 32-bit integers (GPU)",
        32, &timings,
    ));

    // Multiplication
    let timings = run_timed("mul_32bit_gpu", iters, || {
        let _result = server_key.mul(&d_a, &d_b, &streams);
        streams.synchronize();
    });
    all_results.push(summarize(
        "mul_32bit_gpu", "primitive", "A",
        "Multiplication of two encrypted 32-bit integers (GPU)",
        32, &timings,
    ));

    // Comparison (ge)
    let timings = run_timed("cmp_ge_32bit_gpu", iters, || {
        let _result = server_key.ge(&d_a, &d_b, &streams);
        streams.synchronize();
    });
    all_results.push(summarize(
        "cmp_ge_32bit_gpu", "primitive", "A",
        "Greater-or-equal comparison of two encrypted 32-bit integers (GPU)",
        32, &timings,
    ));

    // Scalar addition (encrypted + clear)
    let timings = run_timed("add_32bit_gpu_enc_clear", iters, || {
        let _result = server_key.scalar_add(&d_a, 17u32, &streams);
        streams.synchronize();
    });
    all_results.push(summarize(
        "add_32bit_gpu_enc_clear", "primitive", "A",
        "Addition of encrypted 32-bit + clear scalar (GPU)",
        32, &timings,
    ));

    // Scalar multiplication
    let timings = run_timed("mul_32bit_gpu_enc_clear", iters, || {
        let _result = server_key.scalar_mul(&d_a, 17u32, &streams);
        streams.synchronize();
    });
    all_results.push(summarize(
        "mul_32bit_gpu_enc_clear", "primitive", "A",
        "Multiplication of encrypted 32-bit * clear scalar (GPU)",
        32, &timings,
    ));

    // Division
    let div_iters = std::cmp::min(iters, 3);
    let timings = run_timed("div_32bit_gpu", div_iters, || {
        let _result = server_key.div(&d_a, &d_b, &streams);
        streams.synchronize();
    });
    all_results.push(summarize(
        "div_32bit_gpu", "primitive", "B",
        "Division of two encrypted 32-bit integers (GPU)",
        32, &timings,
    ));

    // ══════════════════════════════════════════════════════════
    // Tier 2: Class A — Simple Capsule Queries (GPU)
    // ══════════════════════════════════════════════════════════
    println!("\n══ Tier 2: Class A — Simple Capsule Queries (GPU) ══\n");

    // Scenario 1: Glucose threshold check (single comparison)
    println!("── Scenario: Glucose threshold check (GPU) ──");
    {
        let glucose_cpu = client_key.encrypt(145u32);
        let threshold_cpu = client_key.encrypt(140u32);
        let d_glucose = CudaUnsignedRadixCiphertext::from_radix_ciphertext(&glucose_cpu, &streams);
        let d_threshold = CudaUnsignedRadixCiphertext::from_radix_ciphertext(&threshold_cpu, &streams);

        let timings = run_timed("glucose_threshold_check_gpu", iters, || {
            let _result = server_key.ge(&d_glucose, &d_threshold, &streams);
            streams.synchronize();
        });
        all_results.push(summarize(
            "glucose_threshold_check_gpu", "class_a", "A",
            "Single encrypted comparison: glucose >= threshold (GPU)",
            32, &timings,
        ));
    }

    // Scenario 2: Credit score range check (two comparisons + bitand)
    println!("\n── Scenario: Credit score range check (GPU) ──");
    {
        let score_cpu = client_key.encrypt(720u32);
        let low_cpu = client_key.encrypt(670u32);
        let high_cpu = client_key.encrypt(740u32);
        let d_score = CudaUnsignedRadixCiphertext::from_radix_ciphertext(&score_cpu, &streams);
        let d_low = CudaUnsignedRadixCiphertext::from_radix_ciphertext(&low_cpu, &streams);
        let d_high = CudaUnsignedRadixCiphertext::from_radix_ciphertext(&high_cpu, &streams);

        let timings = run_timed("credit_range_check_gpu", iters, || {
            let above_low = server_key.ge(&d_score, &d_low, &streams);
            let below_high = server_key.le(&d_score, &d_high, &streams);
            let _in_range = server_key.boolean_bitand(&above_low, &below_high, &streams);
            streams.synchronize();
        });
        all_results.push(summarize(
            "credit_range_check_gpu", "class_a", "A",
            "Two comparisons + AND for range check (GPU)",
            32, &timings,
        ));
    }

    // Scenario 3: Sum of 20 encrypted monthly values
    println!("\n── Scenario: Sum of 20 encrypted monthly values (GPU) ──");
    {
        let monthly_gpu: Vec<CudaUnsignedRadixCiphertext> = (0..20)
            .map(|i| {
                let ct = client_key.encrypt((1000 + i * 50) as u32);
                CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct, &streams)
            })
            .collect();

        let timings = run_timed("sum_20_encrypted_values_gpu", iters, || {
            let mut total = server_key.create_trivial_radix::<_, CudaUnsignedRadixCiphertext>(0u64, num_blocks_32, &streams);
            for val in &monthly_gpu {
                total = server_key.add(&total, val, &streams);
            }
            streams.synchronize();
        });
        all_results.push(summarize(
            "sum_20_encrypted_values_gpu", "class_a", "A",
            "Sum of 20 encrypted 32-bit values (GPU)",
            32, &timings,
        ));
    }

    // Scenario 4: Multi-feature threshold scan (10 features)
    println!("\n── Scenario: Multi-feature threshold scan, 10 features (GPU) ──");
    {
        let features_gpu: Vec<CudaUnsignedRadixCiphertext> = (0..10)
            .map(|i| {
                let ct = client_key.encrypt((50 + i * 10) as u32);
                CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct, &streams)
            })
            .collect();
        let thresholds_gpu: Vec<CudaUnsignedRadixCiphertext> = (0..10)
            .map(|i| {
                let ct = client_key.encrypt((80 + i * 5) as u32);
                CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct, &streams)
            })
            .collect();

        let timings = run_timed("multi_threshold_10_features_gpu", iters, || {
            let mut count = server_key.create_trivial_radix::<_, CudaUnsignedRadixCiphertext>(0u64, num_blocks_32, &streams);
            let one = server_key.create_trivial_radix::<_, CudaUnsignedRadixCiphertext>(1u64, num_blocks_32, &streams);
            let zero = server_key.create_trivial_radix::<_, CudaUnsignedRadixCiphertext>(0u64, num_blocks_32, &streams);
            for (feat, thresh) in features_gpu.iter().zip(thresholds_gpu.iter()) {
                let exceeded = server_key.ge(feat, thresh, &streams);
                let inc = server_key.if_then_else(&exceeded, &one, &zero, &streams);
                count = server_key.add(&count, &inc, &streams);
            }
            streams.synchronize();
        });
        all_results.push(summarize(
            "multi_threshold_10_features_gpu", "class_a", "A",
            "Compare 10 features against thresholds, count exceedances (GPU)",
            32, &timings,
        ));
    }

    // ══════════════════════════════════════════════════════════
    // Tier 3: Class B — Analytical Queries (GPU)
    // ══════════════════════════════════════════════════════════
    println!("\n══ Tier 3: Class B — Analytical Capsule Queries (GPU) ══\n");
    let b_iters = std::cmp::min(iters, 3);

    // Scenario 1: Dot product of 10-element vectors
    println!("── Scenario: Dot product of 10-element encrypted vectors (GPU) ──");
    {
        let vec_a_gpu: Vec<CudaUnsignedRadixCiphertext> = (0..10)
            .map(|i| {
                let ct = client_key.encrypt((10 + i) as u32);
                CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct, &streams)
            })
            .collect();
        let vec_b_gpu: Vec<CudaUnsignedRadixCiphertext> = (0..10)
            .map(|i| {
                let ct = client_key.encrypt((5 + i * 2) as u32);
                CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct, &streams)
            })
            .collect();

        let timings = run_timed("dot_product_10_elem_gpu", b_iters, || {
            let mut sum = server_key.create_trivial_radix::<_, CudaUnsignedRadixCiphertext>(0u64, num_blocks_32, &streams);
            for (a, b) in vec_a_gpu.iter().zip(vec_b_gpu.iter()) {
                let product = server_key.mul(a, b, &streams);
                sum = server_key.add(&sum, &product, &streams);
            }
            streams.synchronize();
        });
        all_results.push(summarize(
            "dot_product_10_elem_gpu", "class_b", "B",
            "Dot product of two 10-element encrypted 32-bit vectors (GPU)",
            32, &timings,
        ));
    }

    // Scenario 2: Weighted scoring of 20 features
    println!("\n── Scenario: Weighted score over 20 encrypted features (GPU) ──");
    {
        let features_gpu: Vec<CudaUnsignedRadixCiphertext> = (0..20)
            .map(|i| {
                let ct = client_key.encrypt((50 + i * 3) as u32);
                CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct, &streams)
            })
            .collect();
        let weights: Vec<u32> = (0..20).map(|i| (1 + i % 5) as u32).collect();

        let timings = run_timed("weighted_score_20_features_gpu", b_iters, || {
            let mut weighted_sum = server_key.create_trivial_radix::<_, CudaUnsignedRadixCiphertext>(0u64, num_blocks_32, &streams);
            for (feat, &w) in features_gpu.iter().zip(weights.iter()) {
                let scaled = server_key.scalar_mul(feat, w, &streams);
                weighted_sum = server_key.add(&weighted_sum, &scaled, &streams);
            }
            streams.synchronize();
        });
        all_results.push(summarize(
            "weighted_score_20_features_gpu", "class_b", "B",
            "Weighted sum of 20 encrypted features with clear weights (GPU)",
            32, &timings,
        ));
    }

    // Scenario 3: Min/Max across 50 encrypted values
    println!("\n── Scenario: Min/Max across 50 encrypted values (GPU) ──");
    {
        let values_gpu: Vec<CudaUnsignedRadixCiphertext> = (0..50)
            .map(|i| {
                let ct = client_key.encrypt((500 + (i * 37) % 2000) as u32);
                CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct, &streams)
            })
            .collect();

        let timings = run_timed("minmax_50_values_gpu", b_iters, || {
            let mut current_min = server_key.add(
                &values_gpu[0],
                &server_key.create_trivial_radix::<_, CudaUnsignedRadixCiphertext>(0u64, num_blocks_32, &streams),
                &streams,
            );
            let mut current_max = server_key.add(
                &values_gpu[0],
                &server_key.create_trivial_radix::<_, CudaUnsignedRadixCiphertext>(0u64, num_blocks_32, &streams),
                &streams,
            );
            for val in &values_gpu[1..] {
                let is_less = server_key.lt(val, &current_min, &streams);
                current_min = server_key.if_then_else(&is_less, val, &current_min, &streams);
                let is_greater = server_key.gt(val, &current_max, &streams);
                current_max = server_key.if_then_else(&is_greater, val, &current_max, &streams);
            }
            streams.synchronize();
        });
        all_results.push(summarize(
            "minmax_50_values_gpu", "class_b", "B",
            "Find min and max across 50 encrypted 32-bit values (GPU)",
            32, &timings,
        ));
    }

    // Scenario 4: Rolling sum of 12 months
    println!("\n── Scenario: 12-month rolling sum (GPU) ──");
    {
        let monthly_gpu: Vec<CudaUnsignedRadixCiphertext> = (0..12)
            .map(|i| {
                let ct = client_key.encrypt((1000 + i * 100) as u32);
                CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct, &streams)
            })
            .collect();

        let timings = run_timed("rolling_sum_12_gpu", b_iters, || {
            let mut total = server_key.add(
                &monthly_gpu[0],
                &server_key.create_trivial_radix::<_, CudaUnsignedRadixCiphertext>(0u64, num_blocks_32, &streams),
                &streams,
            );
            for val in &monthly_gpu[1..] {
                total = server_key.add(&total, val, &streams);
            }
            let _avg = server_key.scalar_div(&total, 12u32, &streams);
            streams.synchronize();
        });
        all_results.push(summarize(
            "rolling_sum_12_gpu", "class_b", "B",
            "Sum 12 encrypted monthly values and divide by 12 (GPU)",
            32, &timings,
        ));
    }

    // ══════════════════════════════════════════════════════════
    // Summary
    // ══════════════════════════════════════════════════════════

    let class_a_benchmarks: Vec<&BenchmarkResult> = all_results
        .iter()
        .filter(|r| r.capsule_class == "A" && r.category.starts_with("class"))
        .collect();
    let class_b_benchmarks: Vec<&BenchmarkResult> = all_results
        .iter()
        .filter(|r| r.capsule_class == "B" && r.category.starts_with("class"))
        .collect();

    let class_a_mean = if class_a_benchmarks.is_empty() {
        0.0
    } else {
        class_a_benchmarks.iter().map(|r| r.mean_ms).sum::<f64>() / class_a_benchmarks.len() as f64
    };
    let class_b_mean = if class_b_benchmarks.is_empty() {
        0.0
    } else {
        class_b_benchmarks.iter().map(|r| r.mean_ms).sum::<f64>() / class_b_benchmarks.len() as f64
    };

    let memory_gb = std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("VmRSS:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|kb| kb / 1_048_576.0)
        .unwrap_or(0.0);

    let summary = ReportSummary {
        total_benchmarks: all_results.len(),
        class_a_viable: class_a_mean < 1000.0,
        class_a_mean_ms: (class_a_mean * 100.0).round() / 100.0,
        class_b_viable: class_b_mean < 30000.0,
        class_b_mean_ms: (class_b_mean * 100.0).round() / 100.0,
        key_gen_seconds: (keygen_ms / 1000.0 * 100.0).round() / 100.0,
        memory_estimate_gb: (memory_gb * 100.0).round() / 100.0,
    };

    let report = BenchmarkReport {
        hardware,
        timestamp: chrono::Utc::now().to_rfc3339(),
        tfhe_version: "1.5.x".to_string(),
        benchmarks: all_results,
        summary,
    };

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  GPU BENCHMARK SUMMARY                                 ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  Total benchmarks:  {:>4}                                ║", report.summary.total_benchmarks);
    println!("║                                                        ║");
    println!("║  Class A (simple queries):                             ║");
    println!("║    Mean latency:    {:>10.2} ms                       ║", report.summary.class_a_mean_ms);
    println!("║    Viable (<1s):    {}                                ║",
        if report.summary.class_a_viable { "YES ✓" } else { "NO ✗ " });
    println!("║                                                        ║");
    println!("║  Class B (analytical):                                 ║");
    println!("║    Mean latency:    {:>10.2} ms                       ║", report.summary.class_b_mean_ms);
    println!("║    Viable (<30s):   {}                                ║",
        if report.summary.class_b_viable { "YES ✓" } else { "NO ✗ " });
    println!("║                                                        ║");
    println!("║  Key generation:    {:>10.2} s                        ║", report.summary.key_gen_seconds);
    println!("║  Memory (RSS):      {:>10.2} GB                      ║", report.summary.memory_estimate_gb);
    println!("╚══════════════════════════════════════════════════════════╝");

    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(&report).expect("Failed to serialize report");
    std::fs::write(&output_path, &json).expect("Failed to write results file");
    println!("\nResults written to: {}", output_path);
}
