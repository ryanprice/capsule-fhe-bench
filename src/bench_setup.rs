use tfhe::prelude::*;
use tfhe::{ConfigBuilder, generate_keys, set_server_key, FheUint16, FheUint32, FheUint64, ClientKey};
use crate::reporting::{run_timed, summarize, BenchmarkResult};
use std::time::Instant;

/// Benchmark one-time setup costs: key generation, encryption, decryption
pub fn bench_setup() -> (Vec<BenchmarkResult>, ClientKey) {
    let mut results = Vec::new();
    let backend = "cpu";

    println!("\n══ Tier 4: Setup & Initialization Costs ══\n");

    // ── Key Generation ──
    println!("── Key generation ──");
    let config = ConfigBuilder::default().build();

    let start = Instant::now();
    let (client_key, server_key) = generate_keys(config);
    let keygen_time = start.elapsed();

    println!("  ✓ Key generation: {:.2}s", keygen_time.as_secs_f64());
    results.push(BenchmarkResult {
        name: "key_generation".to_string(),
        category: "setup".to_string(),
        capsule_class: "setup".to_string(),
        description: "Generate client key + server key (one-time cost per node startup)".to_string(),
        bit_width: 0,
        iterations: 1,
        mean_ms: keygen_time.as_secs_f64() * 1000.0,
        min_ms: keygen_time.as_secs_f64() * 1000.0,
        max_ms: keygen_time.as_secs_f64() * 1000.0,
        std_dev_ms: 0.0,
        backend: backend.to_string(),
    });

    // Set the server key for all subsequent operations
    let start = Instant::now();
    set_server_key(server_key);
    let set_key_time = start.elapsed();

    println!("  ✓ Set server key: {:.2}s", set_key_time.as_secs_f64());
    results.push(BenchmarkResult {
        name: "set_server_key".to_string(),
        category: "setup".to_string(),
        capsule_class: "setup".to_string(),
        description: "Install server key for computation (one-time cost)".to_string(),
        bit_width: 0,
        iterations: 1,
        mean_ms: set_key_time.as_secs_f64() * 1000.0,
        min_ms: set_key_time.as_secs_f64() * 1000.0,
        max_ms: set_key_time.as_secs_f64() * 1000.0,
        std_dev_ms: 0.0,
        backend: backend.to_string(),
    });

    // ── Encryption costs ──
    println!("\n── Encryption latency (per value) ──");

    let timings = run_timed("encrypt_16bit", 50, || {
        let _ = FheUint16::encrypt(12345u16, &client_key);
    });
    results.push(summarize(
        "encrypt_16bit", "setup", "setup",
        "Encrypt a single 16-bit integer", 16, backend, &timings,
    ));

    let timings = run_timed("encrypt_32bit", 50, || {
        let _ = FheUint32::encrypt(123456u32, &client_key);
    });
    results.push(summarize(
        "encrypt_32bit", "setup", "setup",
        "Encrypt a single 32-bit integer", 32, backend, &timings,
    ));

    let timings = run_timed("encrypt_64bit", 20, || {
        let _ = FheUint64::encrypt(123456789u64, &client_key);
    });
    results.push(summarize(
        "encrypt_64bit", "setup", "setup",
        "Encrypt a single 64-bit integer", 64, backend, &timings,
    ));

    // ── Decryption costs ──
    println!("\n── Decryption latency (per value) ──");

    let enc_val_16 = FheUint16::encrypt(12345u16, &client_key);
    let timings = run_timed("decrypt_16bit", 100, || {
        let _: u16 = enc_val_16.decrypt(&client_key);
    });
    results.push(summarize(
        "decrypt_16bit", "setup", "setup",
        "Decrypt a single 16-bit ciphertext", 16, backend, &timings,
    ));

    let enc_val_64 = FheUint64::encrypt(123456789u64, &client_key);
    let timings = run_timed("decrypt_64bit", 50, || {
        let _: u64 = enc_val_64.decrypt(&client_key);
    });
    results.push(summarize(
        "decrypt_64bit", "setup", "setup",
        "Decrypt a single 64-bit ciphertext", 64, backend, &timings,
    ));

    // ── Bulk encryption (simulates capsule creation) ──
    println!("\n── Bulk encryption (capsule creation simulation) ──");

    // Encrypt 100 values (simulates creating a pre-aggregated capsule with 100 features)
    let start = Instant::now();
    let _encrypted_features: Vec<FheUint16> = (0..100)
        .map(|i| FheUint16::encrypt((i * 7 + 42) as u16, &client_key))
        .collect();
    let bulk_time = start.elapsed();

    println!("  ✓ Encrypt 100 x 16-bit values: {:.2}s ({:.2}ms per value)",
        bulk_time.as_secs_f64(),
        bulk_time.as_secs_f64() * 1000.0 / 100.0
    );
    results.push(BenchmarkResult {
        name: "bulk_encrypt_100x16bit".to_string(),
        category: "setup".to_string(),
        capsule_class: "setup".to_string(),
        description: "Encrypt 100 x 16-bit values (simulates capsule creation with 100 pre-aggregated features)".to_string(),
        bit_width: 16,
        iterations: 1,
        mean_ms: bulk_time.as_secs_f64() * 1000.0,
        min_ms: bulk_time.as_secs_f64() * 1000.0,
        max_ms: bulk_time.as_secs_f64() * 1000.0,
        std_dev_ms: 0.0,
        backend: backend.to_string(),
    });

    // ── Memory estimate ──
    println!("\n── Memory usage estimates ──");
    // We can't directly measure TFHE internal memory, but we can report process RSS
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                println!("  Process RSS (after key gen + 100 encryptions): {}", line.trim());
            }
            if line.starts_with("VmPeak:") {
                println!("  Process peak memory: {}", line.trim());
            }
        }
    }

    (results, client_key)
}
