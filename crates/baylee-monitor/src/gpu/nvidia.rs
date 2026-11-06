//! NVML reads telemetry without creating a CUDA context or spawning utilities.
//! It runs on its own thread: a bus throughput reading alone waits about
//! 20 ms in the driver, and a driver in trouble can take far longer.

use crate::types::{Gpu, GpuProcess, GpuView, PcieLink};
use nvml_wrapper::{
    Device, Nvml, bitmasks::device::ThrottleReasons, cuda_driver_version_major,
    cuda_driver_version_minor, enum_wrappers::device::Clock,
    enum_wrappers::device::PcieUtilCounter, enum_wrappers::device::TemperatureSensor,
    enum_wrappers::device::TemperatureThreshold, enums::device::UsedGpuMemory,
};
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

/// What does not change while a driver is loaded, read once per device.
#[derive(Clone, Default)]
struct Fixed {
    name: Option<String>,
    architecture: Option<String>,
    cores: Option<u32>,
    power_limit_default: Option<f64>,
    temp_slowdown: Option<u32>,
    temp_shutdown: Option<u32>,
    clock_graphics_max: Option<u32>,
    clock_memory_max: Option<u32>,
    fans: u32,
    pcie_max_generation: Option<u32>,
    pcie_max_width: Option<u32>,
}

impl Fixed {
    fn read(device: &Device) -> Self {
        Self {
            name: device.name().ok(),
            architecture: device.architecture().ok().map(|value| value.to_string()),
            cores: device.num_cores().ok(),
            power_limit_default: device
                .power_management_limit_default()
                .ok()
                .map(|milliwatts| f64::from(milliwatts) / 1000.0),
            temp_slowdown: device
                .temperature_threshold(TemperatureThreshold::Slowdown)
                .ok(),
            temp_shutdown: device
                .temperature_threshold(TemperatureThreshold::Shutdown)
                .ok(),
            clock_graphics_max: device.max_clock_info(Clock::Graphics).ok(),
            clock_memory_max: device.max_clock_info(Clock::Memory).ok(),
            fans: device.num_fans().unwrap_or(0),
            pcie_max_generation: device.max_pcie_link_gen().ok(),
            pcie_max_width: device.max_pcie_link_width().ok(),
        }
    }
}

pub(super) struct Nvidia {
    nvml: Nvml,
    since: BTreeMap<u32, u64>,
    fixed: BTreeMap<u32, Fixed>,
}

impl Nvidia {
    /// `None` where the library or its driver is missing.
    pub fn new() -> Option<Self> {
        Some(Self {
            nvml: Nvml::init().ok()?,
            since: BTreeMap::new(),
            fixed: BTreeMap::new(),
        })
    }

    pub fn sample(&mut self) -> GpuView {
        let nvml = &self.nvml;
        let Ok(count) = nvml.device_count() else {
            return view(vec![], "GPU-Treiber antwortet nicht".into());
        };
        let mut result = Vec::new();
        for index in 0..count {
            let Ok(device) = nvml.device_by_index(index) else {
                continue;
            };
            let fixed = self
                .fixed
                .entry(index)
                .or_insert_with(|| Fixed::read(&device))
                .clone();
            let since = self.since.entry(index).or_insert(0);
            result.push(read_device(&device, index, &fixed, since));
        }
        let status = if result.len() == count as usize {
            "NVML"
        } else {
            "Einzelne GPUs nicht erreichbar"
        };
        GpuView {
            driver: nvml.sys_driver_version().ok(),
            cuda: nvml.sys_cuda_driver_version().ok().map(|version| {
                format!(
                    "{}.{}",
                    cuda_driver_version_major(version),
                    cuda_driver_version_minor(version)
                )
            }),
            ..view(result, status.into())
        }
    }
}

fn view(gpus: Vec<Gpu>, status: String) -> GpuView {
    GpuView {
        gpus,
        status,
        ..GpuView::default()
    }
}

/// The compute and graphics processes on one device, and how complete that
/// list is. `since` moves on with the newest utilization sample seen.
fn read_processes(device: &Device, index: u32, since: &mut u64) -> (Vec<GpuProcess>, &'static str) {
    let compute = device.running_compute_processes();
    let graphics = device.running_graphics_processes();
    let complete = compute.is_ok() && graphics.is_ok();
    let now_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;
    if *since == 0 {
        *since = now_us.saturating_sub(2_000_000);
    }
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
    let status = if !complete {
        "Prozessliste teilweise nicht verfügbar"
    } else if utilization.is_err() {
        "GPU-Zeit je Prozess nicht verfügbar"
    } else {
        "NVML · fehlende Einzelmessung = unbekannt"
    };
    (processes.into_values().collect(), status)
}

fn read_device(device: &Device, index: u32, fixed: &Fixed, since: &mut u64) -> Gpu {
    let memory = device.memory_info().ok();
    let rates = device.utilization_rates().ok();
    let (processes, process_status) = read_processes(device, index, since);
    let fans: Vec<u32> = (0..fixed.fans)
        .filter_map(|fan| device.fan_speed(fan).ok())
        .collect();
    let clock = |kind| device.clock_info(kind).ok();
    let pcie_rate = |counter| {
        // NVML reports KB/s over a short window.
        device
            .pcie_throughput(counter)
            .ok()
            .map(|kilobytes| f64::from(kilobytes) * 1024.0)
    };
    Gpu {
        index,
        name: fixed.name.clone().unwrap_or_else(|| format!("GPU {index}")),
        architecture: fixed.architecture.clone(),
        cores: fixed.cores,
        utilization: rates.as_ref().map(|v| v.gpu),
        memory_utilization: rates.as_ref().map(|v| v.memory),
        memory_used: memory.as_ref().map(|v| v.used),
        memory_total: memory.as_ref().map(|v| v.total),
        memory_reserved: memory.as_ref().map(|v| v.reserved),
        watts: device.power_usage().ok().map(|v| f64::from(v) / 1000.0),
        power_limit: device
            .enforced_power_limit()
            .ok()
            .map(|v| f64::from(v) / 1000.0),
        power_limit_default: fixed.power_limit_default,
        temperature: device.temperature(TemperatureSensor::Gpu).ok(),
        temp_slowdown: fixed.temp_slowdown,
        temp_shutdown: fixed.temp_shutdown,
        fans,
        clock_graphics: clock(Clock::Graphics),
        clock_sm: clock(Clock::SM),
        clock_memory: clock(Clock::Memory),
        clock_video: clock(Clock::Video),
        clock_graphics_max: fixed.clock_graphics_max,
        clock_memory_max: fixed.clock_memory_max,
        pstate: device
            .performance_state()
            .ok()
            .map(|state| state.as_c())
            .filter(|state| *state <= 15),
        throttle: device
            .current_throttle_reasons()
            .map_or_else(|_| Vec::new(), throttle_codes),
        encoder: device.encoder_utilization().ok().map(|v| v.utilization),
        decoder: device.decoder_utilization().ok().map(|v| v.utilization),
        pcie: PcieLink {
            generation: device.current_pcie_link_gen().ok(),
            width: device.current_pcie_link_width().ok(),
            max_generation: fixed.pcie_max_generation,
            max_width: fixed.pcie_max_width,
        },
        pcie_tx_bps: pcie_rate(PcieUtilCounter::Send),
        pcie_rx_bps: pcie_rate(PcieUtilCounter::Receive),
        pcie_replays: device.pcie_replay_counter().ok(),
        process_status: process_status.into(),
        processes,
        ..Gpu::default()
    }
}

/// Why the clocks are held down, as stable codes for the page. Idling is left
/// out: an idle GPU is not being held back by anything.
fn throttle_codes(reasons: ThrottleReasons) -> Vec<&'static str> {
    [
        (ThrottleReasons::SW_POWER_CAP, "power_cap"),
        (ThrottleReasons::HW_SLOWDOWN, "hw_slowdown"),
        (ThrottleReasons::SW_THERMAL_SLOWDOWN, "sw_thermal"),
        (ThrottleReasons::HW_THERMAL_SLOWDOWN, "hw_thermal"),
        (ThrottleReasons::HW_POWER_BRAKE_SLOWDOWN, "power_brake"),
        (ThrottleReasons::SYNC_BOOST, "sync_boost"),
    ]
    .into_iter()
    .filter(|(reason, _)| reasons.contains(*reason))
    .map(|(_, code)| code)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_reasons_become_codes_and_idle_is_none() {
        assert!(throttle_codes(ThrottleReasons::GPU_IDLE).is_empty());
        assert_eq!(
            throttle_codes(ThrottleReasons::SW_POWER_CAP | ThrottleReasons::HW_THERMAL_SLOWDOWN),
            ["power_cap", "hw_thermal"]
        );
    }
}
