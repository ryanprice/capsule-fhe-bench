use tfhe::prelude::*;
use tfhe::{FheUint16, FheUint32, ClientKey};
use crate::reporting::{run_timed, summarize, BenchmarkResult};

/// Class B: Analytical queries — multi-step computations over encrypted feature vectors
/// These are heavier workloads that research agents and bounty participants would run
pub fn bench_class_b(
    client_key: &ClientKey,
    iterations: u32,
) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
    let backend = "cpu";

    // Reduce iterations for Class B — these are slower
    let iters = std::cmp::min(iterations, 3);

    println!("\n══ Tier 3: Class B — Analytical Capsule Queries ══\n");

    // ── Scenario 1: Dot product of two encrypted 10-element vectors ──
    // Agent: "Compute correlation proxy between glucose and meal timing features"
    println!("── Scenario: Dot product of 10-element encrypted vectors ──");
    {
        let vec_a: Vec<FheUint16> = (0..10)
            .map(|i| FheUint16::encrypt((10 + i) as u16, client_key))
            .collect();
        let vec_b: Vec<FheUint16> = (0..10)
            .map(|i| FheUint16::encrypt((5 + i * 2) as u16, client_key))
            .collect();

        let timings = run_timed("dot_product_10_elem", iters, || {
            let mut sum = FheUint32::encrypt_trivial(0u32);
            for (a, b) in vec_a.iter().zip(vec_b.iter()) {
                // Widen to 32-bit for the product to avoid overflow
                let a_wide: FheUint32 = a.cast_into();
                let b_wide: FheUint32 = b.cast_into();
                let product = &a_wide * &b_wide;
                sum = &sum + &product;
            }
        });
        results.push(summarize(
            "dot_product_10_elem", "class_b", "B",
            "Dot product of two 10-element encrypted 16-bit vectors (widened to 32-bit for products)",
            16, backend, &timings,
        ));
    }

    // ── Scenario 2: Weighted scoring of 20 features ──
    // Agent: "Compute a weighted health risk score from 20 biomarker features"
    // Weights are public (clear), features are encrypted
    println!("\n── Scenario: Weighted score over 20 encrypted features ──");
    {
        let features: Vec<FheUint16> = (0..20)
            .map(|i| FheUint16::encrypt((50 + i * 3) as u16, client_key))
            .collect();
        let weights: Vec<u16> = (0..20).map(|i| (1 + i % 5) as u16).collect();

        let timings = run_timed("weighted_score_20_features", iters, || {
            let mut weighted_sum = FheUint32::encrypt_trivial(0u32);
            for (feat, &w) in features.iter().zip(weights.iter()) {
                let scaled: FheUint32 = feat.cast_into();
                let product = &scaled * (w as u32);
                weighted_sum = &weighted_sum + &product;
            }
        });
        results.push(summarize(
            "weighted_score_20_features", "class_b", "B",
            "Weighted sum of 20 encrypted features with clear weights",
            16, backend, &timings,
        ));
    }

    // ── Scenario 3: Min/Max across 50 encrypted values ──
    // Agent: "Find the minimum and maximum monthly spending from 50 months of data"
    println!("\n── Scenario: Min/Max across 50 encrypted values ──");
    {
        let values: Vec<FheUint32> = (0..50)
            .map(|i| FheUint32::encrypt((500 + (i * 37) % 2000) as u32, client_key))
            .collect();

        let timings = run_timed("minmax_50_values", iters, || {
            let mut current_min = values[0].clone();
            let mut current_max = values[0].clone();
            for val in &values[1..] {
                let is_less = val.lt(&current_min);
                current_min = is_less.if_then_else(val, &current_min);
                let is_greater = val.gt(&current_max);
                current_max = is_greater.if_then_else(val, &current_max);
            }
        });
        results.push(summarize(
            "minmax_50_values", "class_b", "B",
            "Find min and max across 50 encrypted 32-bit values using conditional selection",
            32, backend, &timings,
        ));
    }

    // ── Scenario 4: Moving average proxy (sum of 12 consecutive values) ──
    // Agent: "Compute 12-month rolling sum for trend detection"
    println!("\n── Scenario: 12-month rolling sum ──");
    {
        let monthly: Vec<FheUint32> = (0..12)
            .map(|i| FheUint32::encrypt((1000 + i * 100) as u32, client_key))
            .collect();

        let timings = run_timed("rolling_sum_12", iters, || {
            let mut total = monthly[0].clone();
            for val in &monthly[1..] {
                total = &total + val;
            }
            // Divide by 12 for average
            let _ = &total / 12u32;
        });
        results.push(summarize(
            "rolling_sum_12", "class_b", "B",
            "Sum 12 encrypted monthly values and divide by 12 for rolling average",
            32, backend, &timings,
        ));
    }

    results
}
