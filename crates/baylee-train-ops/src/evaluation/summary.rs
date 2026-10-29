// Counts are bounded by resident JSON input sizes, well below f64's exact 2^53 range.
#![allow(clippy::cast_precision_loss)]
use super::{case_key, flag, number};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Table-cluster bootstrap. The versioned `SplitMix64` stream deliberately avoids
/// depending on `NumPy`'s platform/library RNG implementation. Metrics and censoring
/// rules match the archived Python report; random confidence endpoints may differ.
///
/// # Errors
/// Rejects missing baseline cases, malformed utilities or zero resamples.
pub fn summarize(value: &Value, resamples: usize) -> Result<Value> {
    ensure!(
        value["no_baseline"] != true,
        "A paired house baseline is required"
    );
    ensure!(resamples > 0, "bootstrap resamples must be positive");
    let mut groups = BTreeMap::new();
    let mut caps = 0;
    for row in value["deal_details"]
        .as_array()
        .context("No paired table details")?
    {
        let net = number(row, "net_utility")?;
        let base = number(row, "base_utility")?;
        ensure!(
            (0.0..=1.0).contains(&net) && (0.0..=1.0).contains(&base),
            "utilities must be in [0,1]"
        );
        let nc = flag(row, "net_capped")?;
        let bc = flag(row, "base_capped")?;
        caps += usize::from(nc || bc);
        let entry = groups
            .entry(case_key(row, false)?)
            .or_insert((0usize, 0.0, 0.0, 0.0));
        entry.0 += 1;
        entry.1 += net - base;
        entry.2 += (if nc { 0.0 } else { net }) - (if bc { 1.0 } else { base });
        entry.3 += (if nc { 1.0 } else { net }) - (if bc { 0.0 } else { base });
    }
    ensure!(!groups.is_empty(), "No paired table details");
    let tables: Vec<_> = groups.into_values().collect();
    let count: usize = tables.iter().map(|t| t.0).sum();
    let total = count as f64;
    let estimate = tables.iter().map(|t| t.1).sum::<f64>() / total;
    let lo = tables.iter().map(|t| t.2).sum::<f64>() / total;
    let hi = tables.iter().map(|t| t.3).sum::<f64>() / total;
    let radius = (2.0
        * 40f64.ln()
        * tables
            .iter()
            .map(|t| (t.0 as f64 / total).powi(2))
            .sum::<f64>())
    .sqrt();
    let mut rng = 20_261_023_u64;
    let mut lows = Vec::with_capacity(resamples);
    let mut highs = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let (mut size, mut lower, mut upper) = (0usize, 0.0, 0.0);
        for _ in &tables {
            let t = &tables[sample(&mut rng, tables.len())];
            size += t.0;
            lower += t.2;
            upper += t.3;
        }
        lows.push(lower / size as f64);
        highs.push(upper / size as f64);
    }
    lows.sort_by(f64::total_cmp);
    highs.sort_by(f64::total_cmp);
    let small = tables.len() < 20;
    Ok(json!({"deals": count, "independent_tables":tables.len(),
        "paired_utility_difference": if caps == 0 {Some(estimate)} else {None},
        "paired_difference_with_caps_imputed_half":estimate,
        "censoring_bounds_on_observed_tables":[lo,hi],
        "table_bootstrap_95_percent_interval":[quantile(&lows,0.025),quantile(&highs,0.975)],
        "bounded_independent_table_95_percent_interval":[(lo-radius).max(-1.0),(hi+radius).min(1.0)],
        "capped_pairs":caps,"bootstrap_resamples":resamples,
        "bootstrap_rng":"splitmix64-rejection-v1/20261023",
        "cluster":"table/seed/decks; all focal-seat rotations stay together",
        "scope":"Conditional on these decks, opponent profiles and seeds; not a human-skill claim",
        "caps":"Capped utilities are unknown in [0,1]; interval includes worst/best completion, not a draw",
        "small_sample":small,"warning":if small {Some("Few tables can yield deceptively narrow bootstrap intervals; expand independent tables")} else {None}}))
}
fn sample(state: &mut u64, bound: usize) -> usize {
    let bound = bound as u64;
    let threshold = bound.wrapping_neg() % bound;
    loop {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        if z >= threshold {
            return (z % bound) as usize;
        }
    }
}
fn quantile(values: &[f64], q: f64) -> f64 {
    let x = q * (values.len() - 1) as f64;
    let i = x.floor() as usize;
    values[i] + (values[x.ceil() as usize] - values[i]) * (x - i as f64)
}
