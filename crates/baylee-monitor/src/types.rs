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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        assets,
        procfs::stat::CpuShares,
        sensors::Reading,
        training::{Arm, Eval, Ladder, LlmRun, Pipeline, Progress, Services, Stage, Unit},
    };
    use serde_json::{Value, json};
    use std::collections::BTreeSet;

    #[allow(clippy::too_many_lines)] // One literal, with every field set.
    fn full_snapshot() -> Snapshot {
        let link = PcieLink {
            generation: Some(4),
            width: Some(8),
            max_generation: Some(4),
            max_width: Some(16),
        };
        let usage = Usage {
            total: 100,
            used: 40,
            available: 60,
            used_pct: 40.0,
            inodes_pct: Some(2.0),
            filling_bps: Some(1.0),
            full_in_secs: Some(60),
        };
        let gpu_process = GpuProcess {
            index: 0,
            pid: 42,
            vram: Some(1),
            compute: true,
            graphics: false,
            utilization: Some(3),
        };
        Snapshot {
            point: Point {
                gpu: Some(1.0),
                vram: Some(1.0),
                gpu_watts: Some(1.0),
                cpu_watts: Some(1.0),
                cpu_temp: Some(1.0),
                gpu_temp: Some(1.0),
                psi_cpu: Some(1.0),
                psi_mem: Some(1.0),
                psi_io: Some(1.0),
                disks: vec![("nvme0n1".into(), 1, 2, 3.0)],
                nets: vec![("enp5s0".into(), 1, 2)],
                gpus: vec![(0, Some(1.0), Some(2.0), Some(3.0), Some(4.0))],
                ..Point::default()
            },
            uptime_secs: Some(1),
            load: Some([1.0, 2.0, 3.0]),
            cores: vec![1.0],
            cpu: CpuDetail {
                shares: CpuShares::default(),
                watts: Some(1.0),
                ..CpuDetail::default()
            },
            swap: Swap {
                devices: vec![SwapDevice {
                    name: "swap.img".into(),
                    kind: "file".into(),
                    size: 1,
                    used: 1,
                    priority: -1,
                }],
                ..Swap::default()
            },
            pressure: PressureView {
                cpu: Some(Pressure::default()),
                memory: Some(Pressure::default()),
                io: Some(Pressure::default()),
            },
            gpus: vec![Gpu {
                name: "GPU".into(),
                architecture: Some("Ada".into()),
                cores: Some(1),
                utilization: Some(1),
                memory_utilization: Some(1),
                memory_used: Some(1),
                memory_total: Some(2),
                memory_reserved: Some(1),
                watts: Some(1.0),
                power_limit: Some(1.0),
                power_limit_default: Some(1.0),
                temperature: Some(1),
                temp_slowdown: Some(1),
                temp_shutdown: Some(1),
                fans: vec![1],
                clock_graphics: Some(1),
                clock_sm: Some(1),
                clock_memory: Some(1),
                clock_video: Some(1),
                clock_graphics_max: Some(1),
                clock_memory_max: Some(1),
                pstate: Some(2),
                throttle: vec!["power_cap"],
                encoder: Some(1),
                decoder: Some(1),
                pcie: link.clone(),
                pcie_tx_bps: Some(1.0),
                pcie_rx_bps: Some(1.0),
                pcie_replays: Some(0),
                processes: vec![gpu_process.clone()],
                ..Gpu::default()
            }],
            gpu_driver: Some("595".into()),
            gpu_cuda: Some("13.2".into()),
            sensors: SensorView {
                readings: vec![Reading {
                    chip: "k10temp".into(),
                    label: "Tctl".into(),
                    kind: "temp",
                    value: 50.0,
                    max: Some(90.0),
                    crit: Some(95.0),
                }],
                cpu_temp: Some(50.0),
                freq_mhz: vec![4000],
                freq_min_mhz: Some(2200),
                freq_max_mhz: Some(4100),
                governor: Some("schedutil".into()),
                freq_driver: Some("acpi-cpufreq".into()),
                ..SensorView::default()
            },
            storage: StorageView {
                disks: vec![PhysicalDisk {
                    name: "nvme0n1".into(),
                    temperature: Some(40.0),
                    link: Some(link),
                    io: Some(DiskIo::default()),
                    partitions: vec![Partition {
                        name: "nvme0n1p1".into(),
                        label: Some("xdata".into()),
                        part_name: Some("xdata".into()),
                        fs_type: Some("ext4".into()),
                        mount: Some("/mnt/xdata".into()),
                        usage: Some(usage.clone()),
                        io: Some(DiskIo::default()),
                        ..Partition::default()
                    }],
                    ..PhysicalDisk::default()
                }],
                volumes: vec![Volume {
                    mount: "/tmp".into(),
                    fs_type: "tmpfs".into(),
                    read_only: false,
                    usage: Some(usage),
                }],
                problem: Some("x".into()),
                ..StorageView::default()
            },
            networks: vec![Network {
                name: "enp5s0".into(),
                speed_mbps: Some(10_000),
                duplex: Some("full".into()),
                mtu: Some(1500),
                ..Network::default()
            }],
            processes: vec![Process {
                pid: 42,
                parent: Some(1),
                name: "python3".into(),
                state: "R",
                swap: Some(1),
                threads: Some(1),
                sockets: Some(1),
                gpu: vec![gpu_process],
                ..Process::default()
            }],
            ..Snapshot::default()
        }
    }

    fn full_training() -> Training {
        Training {
            units: vec![Unit {
                name: "baylee-x.service".into(),
                active_state: "active".into(),
                sub_state: "running".into(),
                result: Some("success".into()),
                main_pid: Some(42),
                active_since_ms: Some(1),
                inactive_since_ms: Some(1),
                gone_ms: Some(1),
            }],
            units_problem: Some("x".into()),
            services: Some(Services {
                failed_names: vec!["broken.service".into()],
                ..Services::default()
            }),
            pipelines: vec![Pipeline {
                name: "run".into(),
                problem: Some("x".into()),
                phase: Some("train".into()),
                updated_ms: Some(1),
                promoted: Some(false),
                error: Some("x".into()),
                controller_pid: Some(42),
                controller_alive: Some(true),
                active: true,
                stages: vec![Stage {
                    name: "train".into(),
                    status: Some("running".into()),
                    started_ms: Some(1),
                    finished_ms: Some(1),
                    returncode: Some(0),
                    reason: Some("x".into()),
                    eval: Some(Eval {
                        net_wins: Some(1),
                        ..Eval::default()
                    }),
                    error_line: Some("x".into()),
                }],
                progress: Some(Progress {
                    stage: "train".into(),
                    line: "[t] x=1".into(),
                    fields: vec![("x".into(), "1".into())],
                    step: Some(1),
                    total_steps: Some(2),
                    remaining_s: Some(3),
                    updated_ms: Some(1),
                }),
                ..Pipeline::default()
            }],
            llm_runs: vec![LlmRun {
                models: vec!["m".into()],
                ..LlmRun::default()
            }],
            ladders: vec![Ladder {
                arms: vec![Arm::default()],
                ..Ladder::default()
            }],
            ..Training::default()
        }
    }

    fn sample_json() -> Value {
        let snapshot = full_snapshot();
        let training = full_training();
        serde_json::to_value(Sample::new(&snapshot, &training)).unwrap()
    }

    fn keys(value: &Value, found: &mut BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    found.insert(key.clone());
                    keys(inner, found);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| keys(item, found)),
            _ => {}
        }
    }

    /// The fields the page reads without an underscore in their name; those
    /// with one are checked against the page's source below.
    const PAGE_READS: &[&str] = &[
        "/type",
        "/snapshot/load/2",
        "/snapshot/cores/0",
        "/snapshot/point/cpu",
        "/snapshot/point/ram",
        "/snapshot/point/swap",
        "/snapshot/point/iowait",
        "/snapshot/point/gpu",
        "/snapshot/point/vram",
        "/snapshot/point/disks/0/3",
        "/snapshot/point/nets/0/2",
        "/snapshot/point/gpus/0/4",
        "/snapshot/cpu/usage",
        "/snapshot/cpu/watts",
        "/snapshot/cpu/running",
        "/snapshot/cpu/blocked",
        "/snapshot/cpu/shares/user",
        "/snapshot/cpu/shares/system",
        "/snapshot/cpu/shares/iowait",
        "/snapshot/cpu/shares/irq",
        "/snapshot/memory/total",
        "/snapshot/memory/used",
        "/snapshot/memory/free",
        "/snapshot/memory/available",
        "/snapshot/memory/cached",
        "/snapshot/memory/buffers",
        "/snapshot/memory/shared",
        "/snapshot/memory/slab",
        "/snapshot/memory/dirty",
        "/snapshot/memory/writeback",
        "/snapshot/swap/total",
        "/snapshot/swap/used",
        "/snapshot/swap/free",
        "/snapshot/swap/cached",
        "/snapshot/swap/devices/0/name",
        "/snapshot/swap/devices/0/kind",
        "/snapshot/swap/devices/0/size",
        "/snapshot/swap/devices/0/used",
        "/snapshot/swap/devices/0/priority",
        "/snapshot/pressure/memory/some/0",
        "/snapshot/pressure/memory/full/0",
        "/snapshot/pressure/io/full/0",
        "/snapshot/gpus/0/index",
        "/snapshot/gpus/0/name",
        "/snapshot/gpus/0/architecture",
        "/snapshot/gpus/0/cores",
        "/snapshot/gpus/0/utilization",
        "/snapshot/gpus/0/watts",
        "/snapshot/gpus/0/temperature",
        "/snapshot/gpus/0/fans/0",
        "/snapshot/gpus/0/pstate",
        "/snapshot/gpus/0/throttle/0",
        "/snapshot/gpus/0/encoder",
        "/snapshot/gpus/0/decoder",
        "/snapshot/gpus/0/pcie/generation",
        "/snapshot/gpus/0/pcie/width",
        "/snapshot/gpus/0/processes/0/pid",
        "/snapshot/gpus/0/processes/0/vram",
        "/snapshot/gpus/0/processes/0/utilization",
        "/snapshot/gpus/0/processes/0/compute",
        "/snapshot/sensors/governor",
        "/snapshot/sensors/readings/0/chip",
        "/snapshot/sensors/readings/0/label",
        "/snapshot/sensors/readings/0/kind",
        "/snapshot/sensors/readings/0/value",
        "/snapshot/sensors/readings/0/max",
        "/snapshot/sensors/readings/0/crit",
        "/snapshot/storage/problem",
        "/snapshot/storage/disks/0/name",
        "/snapshot/storage/disks/0/model",
        "/snapshot/storage/disks/0/size",
        "/snapshot/storage/disks/0/transport",
        "/snapshot/storage/disks/0/rotational",
        "/snapshot/storage/disks/0/removable",
        "/snapshot/storage/disks/0/temperature",
        "/snapshot/storage/disks/0/link/generation",
        "/snapshot/storage/disks/0/partitions/0/name",
        "/snapshot/storage/disks/0/partitions/0/label",
        "/snapshot/storage/disks/0/partitions/0/mount",
        "/snapshot/storage/disks/0/partitions/0/size",
        "/snapshot/storage/disks/0/partitions/0/usage/total",
        "/snapshot/storage/disks/0/partitions/0/usage/used",
        "/snapshot/storage/disks/0/partitions/0/usage/available",
        "/snapshot/storage/disks/0/partitions/0/io/read_bps",
        "/snapshot/storage/volumes/0/mount",
        "/snapshot/storage/volumes/0/usage/used_pct",
        "/snapshot/networks/0/name",
        "/snapshot/networks/0/kind",
        "/snapshot/networks/0/state",
        "/snapshot/networks/0/physical",
        "/snapshot/networks/0/duplex",
        "/snapshot/networks/0/mtu",
        "/snapshot/sockets/tcp",
        "/snapshot/sockets/udp",
        "/snapshot/processes/0/pid",
        "/snapshot/processes/0/parent",
        "/snapshot/processes/0/name",
        "/snapshot/processes/0/state",
        "/snapshot/processes/0/cpu",
        "/snapshot/processes/0/ram",
        "/snapshot/processes/0/swap",
        "/snapshot/processes/0/threads",
        "/snapshot/processes/0/sockets",
        "/snapshot/processes/0/gpu/0/index",
        "/training/units/0/name",
        "/training/units/0/result",
        "/training/services/active",
        "/training/services/failed",
        "/training/services/total",
        "/training/pipelines/0/name",
        "/training/pipelines/0/active",
        "/training/pipelines/0/problem",
        "/training/pipelines/0/phase",
        "/training/pipelines/0/promoted",
        "/training/pipelines/0/error",
        "/training/pipelines/0/stages/0/name",
        "/training/pipelines/0/stages/0/status",
        "/training/pipelines/0/stages/0/returncode",
        "/training/pipelines/0/stages/0/reason",
        "/training/pipelines/0/progress/stage",
        "/training/pipelines/0/progress/line",
        "/training/pipelines/0/progress/fields/0/1",
        "/training/pipelines/0/progress/step",
        "/training/pipelines/0/progress/total_steps",
        "/training/pipelines/0/progress/remaining_s",
        "/training/pipelines/0/stages/0/error_line",
        "/training/pipelines/0/stages/0/eval/net_wins",
        "/training/llm_runs/0/models/0",
        "/training/llm_runs/0/name",
        "/training/llm_runs/0/errors",
        "/training/ladders/0/name",
        "/training/ladders/0/arms/0/name",
        "/training/ladders/0/games",
    ];

    #[test]
    fn the_json_holds_everything_the_page_reads() {
        let sample = sample_json();
        for path in PAGE_READS {
            let value = sample.pointer(path);
            assert!(
                value.is_some_and(|value| !value.is_null()),
                "the page reads {path}"
            );
        }
    }

    /// JavaScript reads a renamed field as `undefined` and shows a dash, with
    /// no error anywhere. So every `.snake_case` name in the page's source
    /// must exist on the wire.
    #[test]
    fn every_field_name_in_the_page_exists_in_the_json() {
        let mut wire = BTreeSet::new();
        keys(&sample_json(), &mut wire);
        let facts = Facts {
            hostname: "test".into(),
            os: "test".into(),
            kernel: Some("7.0".into()),
            cpu: "test".into(),
            physical_cores: Some(12),
            threads: 24,
            interval_ms: 1000,
        };
        keys(&serde_json::to_value(facts).unwrap(), &mut wire);
        for (path, text) in assets::scripts() {
            let mut rest = text;
            while let Some(at) = rest.find('.') {
                rest = &rest[at + 1..];
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                let page_only = name.chars().any(|c| c.is_ascii_uppercase());
                if name.contains('_') && !page_only {
                    assert!(wire.contains(&name), "{path} reads .{name}");
                }
            }
        }
    }

    #[test]
    fn the_json_never_carries_a_command_line_a_user_or_an_address() {
        let mut sample = sample_json();
        // `user` here is CPU time spent in user mode, not a person.
        sample["snapshot"]["cpu"]["shares"] = Value::Null;
        let mut wire = BTreeSet::new();
        keys(&sample, &mut wire);
        for forbidden in [
            "cmd",
            "cmdline",
            "command",
            "argv",
            "args",
            "exe",
            "cwd",
            "environ",
            "env",
            "user",
            "username",
            "uid",
            "gid",
            "serial",
            "uuid",
            "mac",
            "address",
            "addresses",
            "ip",
        ] {
            assert!(
                !wire.contains(forbidden),
                "{forbidden} must stay off the wire"
            );
        }
    }

    #[test]
    fn unknown_values_are_null_not_zero() {
        let json = serde_json::to_value(Process::default()).unwrap();
        assert_eq!(json["swap"], Value::Null);
        assert_eq!(json["sockets"], Value::Null);
        let json = serde_json::to_value(Partition::default()).unwrap();
        assert_eq!(json["usage"], Value::Null);
        assert_eq!(json["io"], Value::Null);
    }

    #[test]
    fn history_entries_are_compact_arrays() {
        let json = serde_json::to_value(Point {
            disks: vec![("nvme0n1".into(), 1, 2, 3.5)],
            nets: vec![("lo".into(), 4, 5)],
            gpus: vec![(0, Some(1.0), None, Some(3.0), Some(4.0))],
            ..Point::default()
        })
        .unwrap();
        assert_eq!(json["disks"], json!([["nvme0n1", 1, 2, 3.5]]));
        assert_eq!(json["nets"], json!([["lo", 4, 5]]));
        assert_eq!(json["gpus"], json!([[0, 1.0, null, 3.0, 4.0]]));
    }

    #[test]
    fn tenths_round_and_stay_short_in_json() {
        assert_eq!(serde_json::to_string(&tenths(12.3456)).unwrap(), "12.3");
        assert_eq!(serde_json::to_string(&tenths(99.96)).unwrap(), "100.0");
    }
}
