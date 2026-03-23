use tfhe::prelude::*;
use tfhe::{FheUint16, FheUint32, ClientKey};
use crate::reporting::{run_timed, summarize, BenchmarkResult};

/// Class A: Simple queries over pre-aggregated encrypted capsule features
/// These simulate what an agent would actually request from a health or finance capsule
pub fn bench_class_a(
    client_key: &ClientKey,
    iterations: u32,
) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
    let backend = "cpu";

    println!("\n══ Tier 2: Class A — Simple Capsule Queries ══\n");

    // ── Scenario 1: Glucose threshold check ──
    // Agent asks: "Is this person's average glucose above 140 mg/dL?"
    // This is a single encrypted comparison — the simplest possible capsule query
    println!("── Scenario: Glucose threshold check ──");
    {
        // Pre-aggregated mean glucose (encrypted at capsule creation time)
        let encrypted_mean_glucose = FheUint16::encrypt(135u16, client_key);
        let threshold: u16 = 140;

        let timings = run_timed("glucose_threshold_check", iterations, || {
            let _ = encrypted_mean_glucose.ge(threshold);
        });
        results.push(summarize(
            "glucose_threshold_check", "class_a", "A",
            "Single comparison: is encrypted mean glucose >= 140?", 16, backend, &timings,
        ));
    }

    // ── Scenario 2: Time-in-range calculation ──
    // Agent asks: "What percentage of time was glucose in range 70-180?"
    // Capsule pre-stores count_in_range and total_count as encrypted values
    // Agent computes: (count_in_range * 100) / total_count
    println!("\n── Scenario: Time-in-range (pre-aggregated) ──");
    {
        let count_in_range = FheUint32::encrypt(270_000u32, client_key); // ~90% of 300K
        let total_count = FheUint32::encrypt(300_000u32, client_key);
        let hundred: u32 = 100;

        let timings = run_timed("time_in_range_preagg", iterations, || {
            let numerator = &count_in_range * hundred;
            let _ = &numerator / &total_count;
        });
        results.push(summarize(
            "time_in_range_preagg", "class_a", "A",
            "Compute (count_in_range * 100) / total from pre-aggregated encrypted values", 32, backend, &timings,
        ));
    }

    // ── Scenario 3: Multi-feature threshold scan ──
    // Agent asks: "How many of these 10 health metrics exceed their normal range?"
    // This simulates checking 10 pre-aggregated features against thresholds
    println!("\n── Scenario: Multi-feature threshold scan (10 features) ──");
    {
        let features: Vec<FheUint16> = (0..10)
            .map(|i| FheUint16::encrypt((100 + i * 15) as u16, client_key))
            .collect();
        let thresholds: Vec<u16> = vec![120, 130, 110, 140, 125, 135, 115, 145, 150, 105];

        let timings = run_timed("multi_threshold_10_features", iterations, || {
            let mut count = FheUint16::encrypt(0u16, client_key);
            for (feat, &thresh) in features.iter().zip(thresholds.iter()) {
                // Compare and conditionally add
                let exceeds = feat.ge(thresh);
                // Cast bool to uint and add
                let one_if_exceeds: FheUint16 = exceeds.if_then_else(
                    &FheUint16::encrypt_trivial(1u16),
                    &FheUint16::encrypt_trivial(0u16),
                );
                count = &count + &one_if_exceeds;
            }
        });
        results.push(summarize(
            "multi_threshold_10_features", "class_a", "A",
            "Count how many of 10 encrypted features exceed their thresholds", 16, backend, &timings,
        ));
    }

    // ── Scenario 4: Encrypted sum of 20 pre-aggregated values ──
    // Agent asks: "What is the total across these monthly aggregates?"
    println!("\n── Scenario: Sum of 20 encrypted monthly values ──");
    {
        let monthly_values: Vec<FheUint32> = (0..20)
            .map(|i| FheUint32::encrypt((1000 + i * 50) as u32, client_key))
            .collect();

        let timings = run_timed("sum_20_encrypted_values", iterations, || {
            let mut total = monthly_values[0].clone();
            for val in &monthly_values[1..] {
                total = &total + val;
            }
        });
        results.push(summarize(
            "sum_20_encrypted_values", "class_a", "A",
            "Sum 20 encrypted 32-bit monthly aggregate values", 32, backend, &timings,
        ));
    }

    // ── Scenario 5: Credit score range check ──
    // Agent asks: "Is this person's credit tier above threshold?"
    // Simulates a lending agent checking creditworthiness from a finance capsule
    println!("\n── Scenario: Credit score range check ──");
    {
        let credit_score = FheUint16::encrypt(720u16, client_key);
        let lower_bound: u16 = 680;
        let upper_bound: u16 = 750;

        let timings = run_timed("credit_range_check", iterations, || {
            let above_lower = credit_score.ge(lower_bound);
            let below_upper = credit_score.le(upper_bound);
            let _ = &above_lower & &below_upper; // AND the two conditions
        });
        results.push(summarize(
            "credit_range_check", "class_a", "A",
            "Check if encrypted credit score is within range [680, 750]", 16, backend, &timings,
        ));
    }

    results
}
