//! Package power while a workload runs, from the amdgpu hwmon "PPT" sensor.
//!
//! On a Ryzen APU, amdgpu's `power1_input` (label `PPT`) is the SMU's socket
//! power: CPU cores, iGPU, NPU and the rest of the chip together. It needs no
//! root, unlike the RAPL `energy_uj` counters. The NPU has no power sensor
//! of its own, so its draw shows up as the difference to idle.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

/// The amdgpu hwmon file that reports PPT, if there is one.
pub fn ppt_sensor() -> Option<PathBuf> {
    for entry in fs::read_dir("/sys/class/hwmon").ok()?.flatten() {
        let dir = entry.path();
        let name = fs::read_to_string(dir.join("name")).unwrap_or_default();
        let label = fs::read_to_string(dir.join("power1_label")).unwrap_or_default();
        if name.trim() == "amdgpu" && label.trim() == "PPT" && dir.join("power1_input").exists() {
            return Some(dir.join("power1_input"));
        }
    }
    None
}

fn read_watts(path: &PathBuf) -> Option<f64> {
    let micro: f64 = fs::read_to_string(path).ok()?.trim().parse().ok()?;
    Some(micro / 1e6)
}

/// Mean and peak of the sampled power.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct PowerStats {
    pub mean_w: f64,
    pub max_w: f64,
    pub samples: usize,
}

/// Samples PPT every 20 ms on a thread until stopped.
pub struct Sampler {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<Vec<[f64; 2]>>,
}

impl Sampler {
    pub fn start(sensor: PathBuf) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            let start = std::time::Instant::now();
            let mut series = Vec::new();
            while !flag.load(Ordering::Relaxed) {
                if let Some(w) = read_watts(&sensor) {
                    series.push([start.elapsed().as_secs_f64() * 1e3, w]);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            series
        });
        Self { stop, handle }
    }

    pub fn stop(self) -> Option<PowerStats> {
        self.stop_series().0
    }

    /// The summary and every `[ms since start, watts]` sample.
    pub fn stop_series(self) -> (Option<PowerStats>, Vec<[f64; 2]>) {
        self.stop.store(true, Ordering::Relaxed);
        let series = self.handle.join().unwrap_or_default();
        if series.is_empty() {
            return (None, series);
        }
        let stats = PowerStats {
            mean_w: series.iter().map(|s| s[1]).sum::<f64>() / series.len() as f64,
            max_w: series.iter().map(|s| s[1]).fold(0.0, f64::max),
            samples: series.len(),
        };
        (Some(stats), series)
    }
}
