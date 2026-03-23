use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone)]
pub struct HardwareInfo {
    pub device: String,
    pub cpu: String,
    pub gpu: String,
    pub memory_gb: u64,
    pub os: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub category: String,       // "primitive", "class_a", "class_b", "setup"
    pub capsule_class: String,  // "A", "B", "C", "setup"
    pub description: String,
    pub bit_width: u32,
    pub iterations: u32,
    pub mean_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub std_dev_ms: f64,
    pub backend: String,        // "cpu" or "gpu"
}

#[derive(Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub hardware: HardwareInfo,
    pub timestamp: String,
    pub tfhe_version: String,
    pub benchmarks: Vec<BenchmarkResult>,
    pub summary: ReportSummary,
}

#[derive(Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_benchmarks: usize,
    pub class_a_viable: bool,      // mean < 1000ms
    pub class_a_mean_ms: f64,
    pub class_b_viable: bool,      // mean < 30000ms
    pub class_b_mean_ms: f64,
    pub key_gen_seconds: f64,
    pub memory_estimate_gb: f64,
}

/// Run a benchmark function N times and collect timing statistics
pub fn run_timed<F>(name: &str, iterations: u32, mut f: F) -> Vec<Duration>
where
    F: FnMut() -> (),
{
    let mut timings = Vec::with_capacity(iterations as usize);

    // Warmup run
    println!("  [warmup] {}...", name);
    f();

    for i in 0..iterations {
        if iterations > 5 && i % 5 == 0 {
            println!("  [{}/{}] {}...", i + 1, iterations, name);
        }
        let start = std::time::Instant::now();
        f();
        timings.push(start.elapsed());
    }

    timings
}

/// Convert a vector of durations into a BenchmarkResult
pub fn summarize(
    name: &str,
    category: &str,
    capsule_class: &str,
    description: &str,
    bit_width: u32,
    backend: &str,
    timings: &[Duration],
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
        backend: backend.to_string(),
    }
}
