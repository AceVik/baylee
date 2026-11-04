//! Wire values deliberately exclude command lines, environment and usernames.

use crate::training::Training;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub(crate) struct Facts {
    pub hostname: String,
    pub os: String,
    pub cpu: String,
    pub physical_cores: Option<usize>,
    pub threads: usize,
    pub interval_ms: u64,
}

#[derive(Clone, Default, Serialize)]
pub(crate) struct Point {
    pub at_ms: u64,
    pub cpu: f32,
    pub ram: f64,
    pub gpu: Option<f64>,
    pub vram: Option<f64>,
    pub gpu_watts: Option<f64>,
    pub cpu_watts: Option<f64>,
    pub net_rx: f64,
    pub net_tx: f64,
}

#[derive(Clone, Serialize)]
pub(crate) struct Snapshot {
    pub point: Point,
    pub uptime_secs: Option<u64>,
    pub load: Option<[f64; 3]>,
    pub cores: Vec<f32>,
    pub ram_used: u64,
    pub ram_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub gpus: Vec<Gpu>,
    pub gpu_status: String,
    pub gpu_driver: Option<String>,
    pub gpu_cuda: Option<String>,
    pub cpu_power_status: String,
    pub temperatures: Vec<Temperature>,
    pub disks: Vec<Disk>,
    pub networks: Vec<Network>,
    pub processes: Vec<Process>,
    pub sample_ms: f64,
}

#[derive(Clone, Serialize)]
pub(crate) struct Process {
    pub pid: u32,
    pub name: String,
    /// 100% means one logical CPU. Can exceed 100%.
    pub cpu: f32,
    pub ram: u64,
    pub read_bps: f64,
    pub write_bps: f64,
    pub gpu: Vec<GpuProcess>,
}

#[derive(Clone, Serialize)]
pub(crate) struct GpuProcess {
    pub index: u32,
    pub pid: u32,
    pub vram: Option<u64>,
    pub compute: bool,
    pub graphics: bool,
    pub utilization: Option<u32>,
}

/// A PCI Express link as negotiated and as the hardware could.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub(crate) struct PcieLink {
    pub generation: Option<u32>,
    pub width: Option<u32>,
    pub max_generation: Option<u32>,
    pub max_width: Option<u32>,
}

#[derive(Clone, Serialize)]
pub(crate) struct Gpu {
    pub index: u32,
    pub name: String,
    pub architecture: Option<String>,
    pub cores: Option<u32>,
    pub utilization: Option<u32>,
    /// Time the memory controller was busy, in percent.
    pub memory_utilization: Option<u32>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
    pub memory_reserved: Option<u64>,
    pub watts: Option<f64>,
    pub power_limit: Option<f64>,
    pub power_limit_default: Option<f64>,
    pub temperature: Option<u32>,
    pub temp_slowdown: Option<u32>,
    pub temp_shutdown: Option<u32>,
    pub fan: Option<u32>,
    pub fans: Vec<u32>,
    pub clock_graphics: Option<u32>,
    pub clock_sm: Option<u32>,
    pub clock_memory: Option<u32>,
    pub clock_video: Option<u32>,
    pub clock_graphics_max: Option<u32>,
    pub clock_memory_max: Option<u32>,
    /// The performance state, 0 (fastest) to 15.
    pub pstate: Option<u32>,
    /// Codes for what is holding the clocks down right now.
    pub throttle: Vec<&'static str>,
    pub encoder: Option<u32>,
    pub decoder: Option<u32>,
    pub pcie: PcieLink,
    pub pcie_tx_bps: Option<f64>,
    pub pcie_rx_bps: Option<f64>,
    /// Link errors the hardware had to repeat a transfer for.
    pub pcie_replays: Option<u32>,
    pub process_status: String,
    pub processes: Vec<GpuProcess>,
}

/// The newest result of the GPU thread.
#[derive(Clone, Default)]
pub(crate) struct GpuView {
    pub checked_ms: u64,
    pub gpus: Vec<Gpu>,
    pub status: String,
    pub driver: Option<String>,
    pub cuda: Option<String>,
}

#[derive(Clone, Serialize)]
pub(crate) struct Temperature {
    pub label: String,
    pub celsius: f32,
}

#[derive(Clone, Serialize)]
pub(crate) struct Disk {
    pub mount: String,
    pub total: u64,
    pub available: u64,
}

#[derive(Clone, Serialize)]
pub(crate) struct Network {
    pub name: String,
    pub rx_bps: f64,
    pub tx_bps: f64,
}

/// One live message: the latest snapshot and the training panel's view.
#[derive(Serialize)]
pub(crate) struct Sample<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    snapshot: &'a Snapshot,
    training: &'a Training,
}

impl<'a> Sample<'a> {
    pub fn new(snapshot: &'a Snapshot, training: &'a Training) -> Self {
        Self {
            kind: "sample",
            snapshot,
            training,
        }
    }
}
