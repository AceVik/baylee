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
    handle: JoinHandle<Vec<f64>>,
}

impl Sampler {
    pub fn start(sensor: PathBuf) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            let mut watts = Vec::new();
            while !flag.load(Ordering::Relaxed) {
                if let Some(w) = read_watts(&sensor) {
                    watts.push(w);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            watts
        });
        Self { stop, handle }
    }

    pub fn stop(self) -> Option<PowerStats> {
        self.stop.store(true, Ordering::Relaxed);
        let watts = self.handle.join().ok()?;
        if watts.is_empty() {
            return None;
        }
        Some(PowerStats {
            mean_w: watts.iter().sum::<f64>() / watts.len() as f64,
            max_w: watts.iter().copied().fold(0.0, f64::max),
            samples: watts.len(),
        })
    }
}
