//! Wire values deliberately exclude command lines, environment and usernames.

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
    pub uptime_secs: u64,
    pub load: [f64; 3],
    pub cores: Vec<f32>,
    pub ram_used: u64,
    pub ram_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub gpus: Vec<Gpu>,
    pub gpu_status: String,
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

#[derive(Clone, Serialize)]
pub(crate) struct Gpu {
    pub index: u32,
    pub name: String,
    pub utilization: Option<u32>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
    pub watts: Option<f64>,
    pub power_limit: Option<f64>,
    pub temperature: Option<u32>,
    pub fan: Option<u32>,
    pub process_status: String,
    pub processes: Vec<GpuProcess>,
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
