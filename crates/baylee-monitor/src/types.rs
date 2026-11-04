//! Wire values deliberately exclude command lines, environment and usernames.

use crate::{
    procfs::{pressure::Pressure, sockstat::Sockets, stat::CpuShares, swaps::SwapDevice},
    sensors::SensorView,
    training::Training,
};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub(crate) struct Facts {
    pub hostname: String,
    pub os: String,
    pub kernel: Option<String>,
    pub cpu: String,
    pub physical_cores: Option<usize>,
    pub threads: usize,
    pub interval_ms: u64,
}

/// One moment in the graphs. Thousands of these are kept and sent to every new
/// viewer, so numbers are rounded to what a graph can show: rates to whole
/// bytes per second, percentages and sensors to a tenth.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Point {
    pub at_ms: u64,
    pub cpu: f32,
    pub ram: f32,
    pub swap: f32,
    pub iowait: f32,
    pub gpu: Option<f32>,
    pub vram: Option<f32>,
    pub gpu_watts: Option<f32>,
    pub cpu_watts: Option<f32>,
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub psi_cpu: Option<f32>,
    pub psi_mem: Option<f32>,
    pub psi_io: Option<f32>,
    pub net_rx: u64,
    pub net_tx: u64,
    pub disk_read: u64,
    pub disk_write: u64,
    pub swap_in: u64,
    pub swap_out: u64,
    pub disks: Vec<DiskPoint>,
    pub nets: Vec<NetPoint>,
    pub gpus: Vec<GpuPoint>,
}

/// Name, read and write rate, busy percentage.
pub(crate) type DiskPoint = (String, u64, u64, f32);
/// Name, receive and send rate.
pub(crate) type NetPoint = (String, u64, u64);
/// Index, utilization, memory percentage, watts, temperature.
pub(crate) type GpuPoint = (u32, Option<f32>, Option<f32>, Option<f32>, Option<f32>);

/// Rounds to a tenth, which also keeps the JSON short.
pub(crate) fn tenths(value: f64) -> f32 {
    ((value * 10.0).round() / 10.0) as f32
}

/// Where the CPU's time went and what the scheduler is doing.
#[derive(Clone, Default, Serialize)]
pub(crate) struct CpuDetail {
    /// Percent of all logical CPUs.
    pub usage: f32,
    /// The package's power, where its energy counter is readable.
    pub watts: Option<f64>,
    pub shares: CpuShares,
    pub context_switches_per_s: f64,
    pub interrupts_per_s: f64,
    pub forks_per_s: f64,
    pub running: u32,
    /// Tasks waiting uninterruptibly, usually for a disk.
    pub blocked: u32,
    pub boot_ms: u64,
}

#[derive(Clone, Default, Serialize)]
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

#[derive(Clone, Default, Serialize)]
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
#[derive(Clone, Copy, Default, Serialize)]
pub(crate) struct PressureView {
    pub cpu: Option<Pressure>,
    pub memory: Option<Pressure>,
    pub io: Option<Pressure>,
}

#[derive(Clone, Default, Serialize)]
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
    pub sensors: SensorView,
    pub temperatures: Vec<Temperature>,
    pub disks: Vec<Disk>,
    pub storage: StorageView,
    pub networks: Vec<Network>,
    pub sockets: Sockets,
    pub processes: Vec<Process>,
    pub sample_ms: f64,
}

#[derive(Clone, Default, Serialize)]
pub(crate) struct Process {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    /// The kernel's state letter: `R`, `S`, `D`, `Z`, `T`, `I`, or `?`.
    pub state: &'static str,
    /// 100% means one logical CPU. Can exceed 100%.
    pub cpu: f32,
    pub ram: u64,
    pub swap: Option<u64>,
    pub threads: Option<u32>,
    /// Open sockets; `None` for processes of other users, whose descriptors
    /// are closed to Pulse. Per-process traffic cannot be read at all.
    pub sockets: Option<u32>,
    pub read_bps: f64,
    pub write_bps: f64,
    pub read_total: u64,
    pub write_total: u64,
    /// Unix seconds; zero when unknown.
    pub started_secs: u64,
    pub gpu: Vec<GpuProcess>,
}

#[derive(Clone, Default, Serialize)]
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

#[derive(Clone, Default, Serialize)]
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

#[derive(Clone, Default, Serialize)]
pub(crate) struct Temperature {
    pub label: String,
    pub celsius: f32,
}

#[derive(Clone, Default, Serialize)]
pub(crate) struct Disk {
    pub mount: String,
    pub total: u64,
    pub available: u64,
}

/// How full a filesystem is and where that is heading.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Usage {
    pub total: u64,
    pub used: u64,
    /// What an unprivileged process can still write.
    pub available: u64,
    /// Like `df`: used out of used plus available.
    pub used_pct: f32,
    /// `None` on filesystems without a fixed inode count.
    pub inodes_pct: Option<f32>,
    /// The trend over the last half hour; negative while emptying.
    pub filling_bps: Option<f64>,
    /// When that trend reaches zero free bytes, if it is filling noticeably.
    pub full_in_secs: Option<u64>,
}

/// One device's activity over the last sampling interval.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub(crate) struct DiskIo {
    pub read_bps: f64,
    pub write_bps: f64,
    pub read_iops: f64,
    pub write_iops: f64,
    /// Time with at least one request in progress.
    pub busy_pct: f64,
    pub in_flight: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Partition {
    /// The kernel's device name, such as `nvme1n1p2`.
    pub name: String,
    /// The filesystem's label.
    pub label: Option<String>,
    /// The partition table's own name for it.
    pub part_name: Option<String>,
    pub fs_type: Option<String>,
    pub mount: Option<String>,
    /// Further mounts of the same filesystem, such as bind mounts.
    pub more_mounts: u32,
    pub read_only: bool,
    pub size: u64,
    pub usage: Option<Usage>,
    pub io: Option<DiskIo>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct PhysicalDisk {
    pub name: String,
    pub model: String,
    pub size: u64,
    /// `NVMe`, `SATA`, `USB`, …
    pub transport: String,
    pub rotational: bool,
    pub removable: bool,
    pub temperature: Option<f32>,
    pub link: Option<PcieLink>,
    pub io: Option<DiskIo>,
    pub partitions: Vec<Partition>,
}

/// A local filesystem on no listed partition, such as `/tmp` or a ZFS pool.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Volume {
    pub mount: String,
    pub fs_type: String,
    pub read_only: bool,
    pub usage: Option<Usage>,
}

/// The newest result of the storage thread, plus the sampler's rates.
#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct StorageView {
    /// When this view was assembled; zero before the first refresh.
    pub checked_ms: u64,
    pub disks: Vec<PhysicalDisk>,
    pub volumes: Vec<Volume>,
    pub problem: Option<String>,
}

#[derive(Clone, Default, Serialize)]
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
