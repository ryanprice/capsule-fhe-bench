use tfhe::prelude::*;
use tfhe::{FheUint8, FheUint16, FheUint32, FheUint64, ClientKey, ServerKey};
use crate::reporting::{run_timed, summarize, BenchmarkResult};

/// Benchmark all primitive operations at a given bit width
pub fn bench_primitives(
    client_key: &ClientKey,
    iterations: u32,
) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
    let backend = "cpu";

    println!("\n══ Tier 1: Primitive Operations ══\n");

    // ── 8-bit operations ──
    println!("── 8-bit encrypted integers ──");
    {
        let a = FheUint8::encrypt(42u8, client_key);
        let b = FheUint8::encrypt(17u8, client_key);
        let clear_b: u8 = 17;

        let mut a_clone = a.clone();
        let mut b_clone = b.clone();

        let timings = run_timed("add_8bit_enc_enc", iterations, || {
            let _ = &a + &b;
        });
        results.push(summarize("add_8bit_enc_enc", "primitive", "A", "Addition of two encrypted 8-bit integers", 8, backend, &timings));

        let timings = run_timed("mul_8bit_enc_enc", iterations, || {
            let _ = &a * &b;
        });
        results.push(summarize("mul_8bit_enc_enc", "primitive", "A", "Multiplication of two encrypted 8-bit integers", 8, backend, &timings));

        let timings = run_timed("cmp_ge_8bit_enc_enc", iterations, || {
            let _ = a.ge(&b);
        });
        results.push(summarize("cmp_ge_8bit_enc_enc", "primitive", "A", "Greater-or-equal comparison of two encrypted 8-bit integers", 8, backend, &timings));

        let timings = run_timed("add_8bit_enc_clear", iterations, || {
            let _ = &a + clear_b;
        });
        results.push(summarize("add_8bit_enc_clear", "primitive", "A", "Addition of encrypted 8-bit + clear 8-bit", 8, backend, &timings));
    }

    // ── 16-bit operations ──
    println!("\n── 16-bit encrypted integers ──");
    {
        let a = FheUint16::encrypt(4200u16, client_key);
        let b = FheUint16::encrypt(1700u16, client_key);
        let clear_b: u16 = 1700;

        let timings = run_timed("add_16bit_enc_enc", iterations, || {
            let _ = &a + &b;
        });
        results.push(summarize("add_16bit_enc_enc", "primitive", "A", "Addition of two encrypted 16-bit integers", 16, backend, &timings));

        let timings = run_timed("mul_16bit_enc_enc", iterations, || {
            let _ = &a * &b;
        });
        results.push(summarize("mul_16bit_enc_enc", "primitive", "A", "Multiplication of two encrypted 16-bit integers", 16, backend, &timings));

        let timings = run_timed("cmp_ge_16bit_enc_enc", iterations, || {
            let _ = a.ge(&b);
        });
        results.push(summarize("cmp_ge_16bit_enc_enc", "primitive", "A", "Greater-or-equal comparison of two encrypted 16-bit integers", 16, backend, &timings));
    }

    // ── 32-bit operations ──
    println!("\n── 32-bit encrypted integers ──");
    {
        let a = FheUint32::encrypt(420_000u32, client_key);
        let b = FheUint32::encrypt(170_000u32, client_key);

        let timings = run_timed("add_32bit_enc_enc", iterations, || {
            let _ = &a + &b;
        });
        results.push(summarize("add_32bit_enc_enc", "primitive", "A", "Addition of two encrypted 32-bit integers", 32, backend, &timings));

        let timings = run_timed("mul_32bit_enc_enc", iterations, || {
            let _ = &a * &b;
        });
        results.push(summarize("mul_32bit_enc_enc", "primitive", "B", "Multiplication of two encrypted 32-bit integers", 32, backend, &timings));

        let timings = run_timed("cmp_ge_32bit_enc_enc", iterations, || {
            let _ = a.ge(&b);
        });
        results.push(summarize("cmp_ge_32bit_enc_enc", "primitive", "A", "Greater-or-equal comparison of two encrypted 32-bit integers", 32, backend, &timings));
    }

    // ── 64-bit operations ──
    println!("\n── 64-bit encrypted integers ──");
    {
        let a = FheUint64::encrypt(42_000_000u64, client_key);
        let b = FheUint64::encrypt(17_000_000u64, client_key);

        let timings = run_timed("add_64bit_enc_enc", iterations, || {
            let _ = &a + &b;
        });
        results.push(summarize("add_64bit_enc_enc", "primitive", "A", "Addition of two encrypted 64-bit integers", 64, backend, &timings));

        let timings = run_timed("mul_64bit_enc_enc", iterations, || {
            let _ = &a * &b;
        });
        results.push(summarize("mul_64bit_enc_enc", "primitive", "B", "Multiplication of two encrypted 64-bit integers", 64, backend, &timings));

        let timings = run_timed("cmp_ge_64bit_enc_enc", iterations, || {
            let _ = a.ge(&b);
        });
        results.push(summarize("cmp_ge_64bit_enc_enc", "primitive", "A", "Greater-or-equal comparison of two encrypted 64-bit integers", 64, backend, &timings));

        // Division is the most expensive — fewer iterations
        let div_iters = std::cmp::min(iterations, 5);
        let timings = run_timed("div_64bit_enc_enc", div_iters, || {
            let _ = &a / &b;
        });
        results.push(summarize("div_64bit_enc_enc", "primitive", "C", "Division of two encrypted 64-bit integers", 64, backend, &timings));
    }

    results
}
