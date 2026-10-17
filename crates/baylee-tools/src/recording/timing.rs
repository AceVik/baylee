//! Recorded timelines, including cumulative quantization without FPS drift.
use super::cli::Cadence;
use super::data::Recording;
use anyhow::{Result, ensure};

pub(super) fn delays(recording: &Recording, cadence: Cadence, quantum_ms: u32) -> Result<Vec<u32>> {
    if cadence == Cadence::Step {
        let step_ms = 1000_u32.div_ceil(quantum_ms) * quantum_ms;
        return Ok(vec![step_ms; recording.frames.len()]);
    }
    let times: Vec<_> = recording
        .frames
        .iter()
        .map(|frame| match cadence {
            Cadence::Real => frame.requested_us,
            Cadence::Virtual => frame.virtual_us,
            Cadence::Nominal => frame.nominal_us,
            Cadence::Step => unreachable!(),
        })
        .collect();
    let mut differences = Vec::with_capacity(times.len());
    for pair in times.windows(2) {
        ensure!(
            pair[1] > pair[0],
            "{} times do not increase; use real or nominal cadence",
            cadence.name()
        );
        differences.push(pair[1] - pair[0]);
    }
    let fps = recording.summary["options"]["fps"]
        .as_u64()
        .filter(|fps| (1..=60).contains(fps))
        .ok_or_else(|| anyhow::anyhow!("invalid nominal fps"))?;
    let mut last = differences
        .last()
        .copied()
        .unwrap_or_else(|| round_ratio(1_000_000, fps));
    if cadence == Cadence::Real && !recording.summary["capture_end_us"].is_null() {
        let end = recording.summary["capture_end_us"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("invalid capture_end_us"))?;
        last = end.saturating_sub(*times.last().unwrap()).max(1);
    }
    differences.push(last);
    ensure!(
        differences.iter().all(|delta| *delta <= 60_000_000),
        "a frame delay exceeds 60 seconds"
    );
    let quantum_us = u64::from(quantum_ms) * 1000;
    let mut elapsed = 0_u64;
    let mut emitted_ticks = 0_u64;
    Ok(differences
        .into_iter()
        .map(|delta| {
            elapsed += delta;
            let target = (emitted_ticks + 1).max((elapsed + quantum_us / 2) / quantum_us);
            let duration = (target - emitted_ticks) * u64::from(quantum_ms);
            emitted_ticks = target;
            u32::try_from(duration).expect("at most 60 seconds per sample")
        })
        .collect())
}

/// Integer ties-to-even, matching Python's round without floating-point loss.
pub(super) fn round_ratio(numerator: u64, denominator: u64) -> u64 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    quotient
        + u64::from(
            remainder > denominator / 2 || (remainder * 2 == denominator && quotient % 2 == 1),
        )
}

pub(super) fn sheet_indices(count: usize, requested: u32) -> Vec<usize> {
    let take = count.min(requested as usize);
    if take <= 1 {
        return vec![0];
    }
    (0..take)
        .map(|i| round_ratio((i * (count - 1)) as u64, (take - 1) as u64) as usize)
        .collect()
}
