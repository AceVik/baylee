//! NVML reads telemetry without creating a CUDA context or spawning utilities.

use crate::types::{Gpu, GpuProcess};
use nvml_wrapper::{Nvml, enum_wrappers::device::TemperatureSensor, enums::device::UsedGpuMemory};
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) struct Gpus {
    nvml: Option<Nvml>,
    unavailable: String,
    since: BTreeMap<u32, u64>,
}

impl Gpus {
    pub fn new() -> Self {
        match Nvml::init() {
            Ok(nvml) => Self { nvml: Some(nvml), unavailable: String::new(), since: BTreeMap::new() },
            Err(_) => Self { nvml: None, unavailable: "NVIDIA-Telemetrie nicht verfügbar; andere GPU-Hersteller noch nicht unterstützt".into(), since: BTreeMap::new() },
        }
    }

    pub fn sample(&mut self) -> (Vec<Gpu>, String) {
        let Some(nvml) = &self.nvml else {
            return (vec![], self.unavailable.clone());
        };
        let Ok(count) = nvml.device_count() else {
            return (vec![], "GPU-Treiber antwortet nicht".into());
        };
        let mut result = Vec::new();
        for index in 0..count {
            let Ok(device) = nvml.device_by_index(index) else {
                continue;
            };
            let memory = device.memory_info().ok();
            let compute = device.running_compute_processes();
            let graphics = device.running_graphics_processes();
            let complete = compute.is_ok() && graphics.is_ok();
            let now_us = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64;
            let since = self
                .since
                .entry(index)
                .or_insert(now_us.saturating_sub(2_000_000));
            let utilization = device.process_utilization_stats(Some(*since));
            let mut by_pid = BTreeMap::new();
            if let Ok(samples) = &utilization {
                for sample in samples {
                    *since = (*since).max(sample.timestamp);
                    // Never repeat a stale busy value after a process goes idle.
                    if now_us.saturating_sub(sample.timestamp) < 2_500_000 {
                        let item = by_pid.entry(sample.pid).or_insert((0, 0));
                        if sample.timestamp >= item.0 {
                            *item = (sample.timestamp, sample.sm_util);
                        }
                    }
                }
            }
            let mut processes = BTreeMap::<u32, GpuProcess>::new();
            for (is_compute, list) in [(true, compute), (false, graphics)] {
                for process in list.unwrap_or_default() {
                    let bytes = match process.used_gpu_memory {
                        UsedGpuMemory::Used(value) => Some(value),
                        UsedGpuMemory::Unavailable => None,
                    };
                    let entry = processes.entry(process.pid).or_insert(GpuProcess {
                        index,
                        pid: process.pid,
                        vram: bytes,
                        compute: false,
                        graphics: false,
                        utilization: by_pid.get(&process.pid).map(|(_, value)| *value),
                    });
                    // The same context can appear in both lists. Do not sum twice.
                    entry.vram = entry.vram.max(bytes);
                    entry.compute |= is_compute;
                    entry.graphics |= !is_compute;
                }
            }
            result.push(Gpu {
                index,
                name: device.name().unwrap_or_else(|_| format!("GPU {index}")),
                utilization: device.utilization_rates().ok().map(|v| v.gpu),
                memory_used: memory.as_ref().map(|v| v.used),
                memory_total: memory.as_ref().map(|v| v.total),
                watts: device.power_usage().ok().map(|v| f64::from(v) / 1000.0),
                power_limit: device
                    .power_management_limit()
                    .ok()
                    .map(|v| f64::from(v) / 1000.0),
                temperature: device.temperature(TemperatureSensor::Gpu).ok(),
                fan: device.fan_speed(0).ok(),
                process_status: if !complete {
                    "Prozessliste teilweise nicht verfügbar"
                } else if utilization.is_err() {
                    "GPU-Zeit je Prozess nicht verfügbar"
                } else {
                    "NVML · fehlende Einzelmessung = unbekannt"
                }
                .into(),
                processes: processes.into_values().collect(),
            });
        }
        let status = if result.len() == count as usize {
            "NVML"
        } else {
            "Einzelne GPUs nicht erreichbar"
        };
        (result, status.into())
    }
}
