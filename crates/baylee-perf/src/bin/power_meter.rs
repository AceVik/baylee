//! Mean power and joules per run of a command, read off a sensor file.
//!
//! ```text
//! rustc -O crates/baylee-perf/src/bin/power_meter.rs -o power_meter
//! power_meter /sys/class/hwmon/hwmon8/power1_input 30 -- taskset -c 2 ./engine_perf house
//! power_meter /sys/class/hwmon/hwmon8/power1_input 30
//! ```
//!
//! For a machine whose energy counter a normal account may not read
//! (`perf_event_paranoid`, `energy_uj` readable by root alone) but whose power
//! sensor it may. The command runs again and again for the whole window while
//! the sensor (microwatts) is sampled ten times a second, because a sensor is an
//! estimate and a short run sees mostly its noise. With no command it reads the
//! machine at rest, which is the part of every figure no build can change.
//!
//! This is a sampled estimate of power and not a count of energy: compare builds
//! with it, measured the same way in alternating rounds, and read the spread.
//! Standard library alone, so plain `rustc` builds it.

use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// How often the sensor is read.
const SAMPLE: Duration = Duration::from_millis(100);

/// Reads `path` until `stop`, in watts.
fn sample(path: &str, stop: &AtomicBool) -> Result<Vec<f64>, String> {
    let mut watts = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let microwatts: f64 = text
            .trim()
            .parse()
            .map_err(|e| format!("{path} holds no number: {e}"))?;
        watts.push(microwatts / 1e6);
        std::thread::sleep(SAMPLE);
    }
    Ok(watts)
}

/// Runs `command` to the end, over and over until `window` is up.
fn repeat(command: &[String], window: Duration) -> Result<(u32, Duration), String> {
    let started = Instant::now();
    let mut runs = 0;
    while started.elapsed() < window {
        let status = Command::new(&command[0])
            .args(&command[1..])
            .stdout(Stdio::null())
            .status()
            .map_err(|e| format!("{}: {e}", command[0]))?;
        if !status.success() {
            return Err(format!("{} ended with {status}", command[0]));
        }
        runs += 1;
    }
    Ok((runs, started.elapsed()))
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: power_meter SENSOR SECONDS [-- COMMAND...]";
    let sensor = args.first().ok_or(usage)?.clone();
    let seconds: u64 = args
        .get(1)
        .ok_or(usage)?
        .parse()
        .map_err(|e| format!("seconds: {e}"))?;
    let window = Duration::from_secs(seconds);
    let command = args.get(3..).unwrap_or_default();

    let stop = Arc::new(AtomicBool::new(false));
    let sampler = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || sample(&sensor, &stop))
    };
    let played = if command.is_empty() {
        std::thread::sleep(window);
        Ok((0, window))
    } else {
        repeat(command, window)
    };
    stop.store(true, Ordering::Relaxed);
    let watts = sampler.join().map_err(|_| "the sampler panicked")??;
    let (runs, elapsed) = played?;

    let mean =
        watts.iter().sum::<f64>() / f64::from(u32::try_from(watts.len()).unwrap_or(1).max(1));
    let total = mean * elapsed.as_secs_f64();
    let per_run = if runs == 0 {
        0.0
    } else {
        total / f64::from(runs)
    };
    println!(
        "runs={runs} seconds={:.2} samples={} mean_w={mean:.2} joules_per_run={per_run:.2}",
        elapsed.as_secs_f64(),
        watts.len(),
    );
    Ok(())
}
