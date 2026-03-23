mod reporting;
mod bench_primitives;
mod bench_class_a;
mod bench_class_b;
mod bench_setup;

use reporting::{BenchmarkReport, HardwareInfo, ReportSummary};
use std::fs;

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

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  CAPSULE PROTOCOL — FHE BENCHMARK SUITE                ║");
    println!("║  Target: NVIDIA DGX Spark / GB10 Grace Blackwell       ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Parse args
    let args: Vec<String> = std::env::args().collect();
    let output_path = args.iter()
        .position(|a| a == "--output")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "results/bench_results.json".to_string());

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

    // Collect all results
    let mut all_results = Vec::new();

    // ── Tier 4: Setup (runs first, also gives us the keys) ──
    let (setup_results, client_key) = bench_setup::bench_setup();
    all_results.extend(setup_results);

    // ── Tier 1: Primitives ──
    let prim_results = bench_primitives::bench_primitives(&client_key, iters);
    all_results.extend(prim_results);

    // ── Tier 2: Class A ──
    let class_a_results = bench_class_a::bench_class_a(&client_key, iters);
    all_results.extend(class_a_results);

    // ── Tier 3: Class B ──
    let class_b_results = bench_class_b::bench_class_b(&client_key, iters);
    all_results.extend(class_b_results);

    // ── Compute Summary ──
    let class_a_benchmarks: Vec<&reporting::BenchmarkResult> = all_results
        .iter()
        .filter(|r| r.capsule_class == "A" && r.category.starts_with("class"))
        .collect();
    let class_b_benchmarks: Vec<&reporting::BenchmarkResult> = all_results
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

    let key_gen_ms = all_results
        .iter()
        .find(|r| r.name == "key_generation")
        .map(|r| r.mean_ms)
        .unwrap_or(0.0);

    // Read memory
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
        key_gen_seconds: (key_gen_ms / 1000.0 * 100.0).round() / 100.0,
        memory_estimate_gb: (memory_gb * 100.0).round() / 100.0,
    };

    // ── Build Report ──
    let report = BenchmarkReport {
        hardware,
        timestamp: chrono::Utc::now().to_rfc3339(),
        tfhe_version: "1.5.x".to_string(),
        benchmarks: all_results,
        summary,
    };

    // ── Print Summary ──
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  BENCHMARK SUMMARY                                     ║");
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

    // ── Write JSON ──
    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(&report).expect("Failed to serialize report");
    fs::write(&output_path, &json).expect("Failed to write results file");
    println!("\nResults written to: {}", output_path);
}
