//! Wire values deliberately exclude command lines, environment and usernames.

use crate::{
    procfs::{pressure::Pressure, sockstat::Sockets, stat::CpuShares, swaps::SwapDevice},
    training::Training,
};
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
    pub swap: f64,
    pub swap_in: u64,
    pub swap_out: u64,
    pub iowait: f32,
    pub psi_cpu: Option<f32>,
    pub psi_mem: Option<f32>,
    pub psi_io: Option<f32>,
}

/// Where the CPU's time went and what the scheduler is doing.
#[derive(Clone, Serialize)]
pub(crate) struct CpuDetail {
    pub shares: CpuShares,
    pub context_switches_per_s: f64,
    pub interrupts_per_s: f64,
    pub forks_per_s: f64,
    pub running: u32,
    /// Tasks waiting uninterruptibly, usually for a disk.
    pub blocked: u32,
    pub boot_ms: u64,
}

#[derive(Clone, Serialize)]
pub(crate) struct Memory {
    pub total: u64,
    /// Not available for new work, by the OS's own accounting.
    pub used: u64,
    pub available: u64,
    pub free: u64,
    pub buffers: u64,
    pub cached: u64,
    pub shared: u64,
    pub slab: u64,
    pub dirty: u64,
    pub writeback: u64,
    pub major_faults_per_s: f64,
    /// Processes the kernel's OOM killer has ended since boot.
    pub oom_kills: u64,
}

#[derive(Clone, Serialize)]
pub(crate) struct Swap {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub cached: u64,
    pub in_bps: f64,
    pub out_bps: f64,
    pub devices: Vec<SwapDevice>,
}

/// Time lost waiting for each resource; `None` where the kernel does not say.
#[derive(Clone, Copy, Serialize)]
pub(crate) struct PressureView {
    pub cpu: Option<Pressure>,
    pub memory: Option<Pressure>,
    pub io: Option<Pressure>,
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
    pub cpu: CpuDetail,
    pub memory: Memory,
    pub swap: Swap,
    pub pressure: PressureView,
    pub gpus: Vec<Gpu>,
    pub gpu_status: String,
    pub gpu_driver: Option<String>,
    pub gpu_cuda: Option<String>,
    pub cpu_power_status: String,
    pub temperatures: Vec<Temperature>,
    pub disks: Vec<Disk>,
    pub networks: Vec<Network>,
    pub sockets: Sockets,
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
    /// `ethernet`, `wlan`, `bridge`, `tunnel`, `loopback` or `virtual`.
    pub kind: String,
    /// The kernel's `operstate`.
    pub state: String,
    /// A real adapter, counted in the machine's total traffic.
    pub physical: bool,
    pub speed_mbps: Option<u32>,
    pub duplex: Option<String>,
    pub mtu: Option<u32>,
    pub rx_bps: f64,
    pub tx_bps: f64,
    pub rx_pps: f64,
    pub tx_pps: f64,
    /// Bytes since the interface appeared, normally since boot.
    pub rx_total: u64,
    pub tx_total: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_drops: u64,
    pub tx_drops: u64,
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
