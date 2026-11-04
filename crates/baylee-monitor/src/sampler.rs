//! One shared, blocking sampler. Browser count never changes collection work.
//! It reads only what cannot block on hardware (`/proc` and the process list)
//! and takes the rest from the slow sources' threads.
#![allow(clippy::cast_precision_loss)] // Percentages/rates, not exact byte accounting.

use crate::{
    background::{self, Latest},
    gpu::Gpus,
    network::{self, Link},
    power::Power,
    processes::{self, Extra},
    procfs::{
        meminfo, per_second, pressure, sockstat,
        stat::{self, Stat},
        swaps,
        vmstat::{self, Vmstat},
    },
    sensors::{self, SensorView},
    storage::{self, Storage},
    types::{
        CpuDetail, Disk, Facts, Gpu, GpuView, Memory, Point, PressureView, Process, Snapshot,
        StorageView, Swap, Temperature, tenths,
    },
};
use anyhow::Context;
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Duration, Instant},
};
use sysinfo::{ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System};

/// Link speed and state change rarely, and asking the driver can be slow.
const LINKS_EVERY: Duration = Duration::from_secs(5);
/// Temperatures and fans move over seconds; an embedded controller is slow.
const SENSORS_EVERY: Duration = Duration::from_secs(2);
/// Often enough that a filling disk shows within seconds; `statvfs` and the
/// sysfs walk are its own thread's business, so a sick disk stalls only this.
const STORAGE_EVERY: Duration = Duration::from_secs(5);
/// Threads, swap and sockets per process are details, not a 1 Hz signal.
const PROCESSES_EVERY: Duration = Duration::from_secs(3);
/// Interfaces kept per history point. The busiest ones come first in `networks`.
const MAX_HISTORY_NETS: usize = 12;

pub(crate) struct Sampler {
    system: System,
    network: network::Tracker,
    links: Latest<BTreeMap<String, Link>>,
    sensors: Latest<SensorView>,
    extras: Latest<BTreeMap<u32, Extra>>,
    storage: Latest<StorageView>,
    disk_rates: storage::rates::Tracker,
    gpu: Latest<GpuView>,
    gpu_stale_ms: u64,
    power: Power,
    previous: Instant,
    stat: Stat,
    vmstat: Vmstat,
    page_size: u64,
}

impl Sampler {
    pub fn new(interval: Duration) -> anyhow::Result<Self> {
        let mut system = System::new().context("System telemetry unavailable")?;
        system.refresh_cpu_all();
        refresh_processes(&mut system);
        let gpu = Latest::new(GpuView::default());
        let mut gpus = Gpus::new();
        background::spawn("gpu", interval, gpu.clone(), move || gpus.sample())?;
        let threads = system.cpus().len();
        let sensors = Latest::new(SensorView::default());
        background::spawn("sensors", SENSORS_EVERY, sensors.clone(), move || {
            sensors::read(threads)
        })?;
        let extras = Latest::new(BTreeMap::new());
        background::spawn("processes", PROCESSES_EVERY, extras.clone(), || {
            processes::scan(Path::new("/proc"))
        })?;
        let storage = Latest::new(StorageView::default());
        let mut disks = Storage::default();
        background::spawn("storage", STORAGE_EVERY, storage.clone(), move || {
            disks.refresh()
        })?;
        let links = Latest::new(BTreeMap::new());
        background::spawn("links", LINKS_EVERY, links.clone(), || {
            network::read_links(Path::new("/sys/class/net"))
        })?;
        Ok(Self {
            system,
            sensors,
            network: network::Tracker::default(),
            links,
            extras,
            storage,
            disk_rates: storage::rates::Tracker::default(),
            gpu,
            gpu_stale_ms: (5 * interval.as_millis() as u64).max(5000),
            power: Power::new(),
            previous: Instant::now(),
            stat: stat::parse(&read_text("/proc/stat")),
            vmstat: vmstat::parse(&read_text("/proc/vmstat")),
            page_size: rustix::param::page_size() as u64,
        })
    }

    pub fn facts(&self, interval_ms: u64) -> Facts {
        Facts {
            hostname: System::host_name().unwrap_or_else(|_| "Workstation".into()),
            os: System::long_os_version().unwrap_or_else(|_| std::env::consts::OS.into()),
            kernel: System::kernel_version().ok(),
            cpu: self
                .system
                .cpus()
                .first()
                .map_or("CPU", sysinfo::Cpu::brand)
                .into(),
            physical_cores: System::physical_core_count().ok(),
            threads: self.system.cpus().len(),
            interval_ms,
        }
    }

    /// The GPU thread's newest view. A driver that stopped answering gives
    /// nothing at all: its last busy value would be a lie.
    fn fresh_gpu_view(&self) -> GpuView {
        let view = self.gpu.get();
        if background::now_ms().saturating_sub(view.checked_ms) <= self.gpu_stale_ms {
            (*view).clone()
        } else {
            GpuView {
                status: "GPU-Treiber antwortet nicht".into(),
                ..GpuView::default()
            }
        }
    }

    fn read_memory(&mut self, elapsed: f64) -> (Memory, Swap) {
        let info = meminfo::parse(&read_text("/proc/meminfo"));
        let now = vmstat::parse(&read_text("/proc/vmstat"));
        let before = std::mem::replace(&mut self.vmstat, now);
        let memory = Memory {
            total: info.total,
            used: info.used(),
            available: info.available,
            free: info.free,
            buffers: info.buffers,
            cached: info.cached,
            shared: info.shared,
            slab: info.slab,
            dirty: info.dirty,
            writeback: info.writeback,
            major_faults_per_s: per_second(before.major_faults, now.major_faults, elapsed),
            oom_kills: now.oom_kills,
        };
        let page = self.page_size as f64;
        let swap = Swap {
            total: info.swap_total,
            used: info.swap_used(),
            free: info.swap_free,
            cached: info.swap_cached,
            in_bps: per_second(before.swap_in, now.swap_in, elapsed) * page,
            out_bps: per_second(before.swap_out, now.swap_out, elapsed) * page,
            devices: swaps::parse(&read_text("/proc/swaps")),
        };
        (memory, swap)
    }

    fn read_cpu(&mut self, elapsed: f64, watts: Option<f64>) -> CpuDetail {
        let now = stat::parse(&read_text("/proc/stat"));
        let before = std::mem::replace(&mut self.stat, now);
        CpuDetail {
            usage: self.system.global_cpu_usage(),
            watts,
            shares: stat::shares(&before.cpu, &now.cpu),
            context_switches_per_s: per_second(
                before.context_switches,
                now.context_switches,
                elapsed,
            ),
            interrupts_per_s: per_second(before.interrupts, now.interrupts, elapsed),
            forks_per_s: per_second(before.forks, now.forks, elapsed),
            running: now.running,
            blocked: now.blocked,
            boot_ms: now.boot_secs.saturating_mul(1000),
        }
    }

    pub fn sample(&mut self) -> Snapshot {
        let started = Instant::now();
        let elapsed = started
            .duration_since(self.previous)
            .as_secs_f64()
            .max(0.001);
        self.previous = started;
        self.system.refresh_cpu_usage();
        refresh_processes(&mut self.system);
        let gpu = self.fresh_gpu_view();
        let (cpu_watts, cpu_power_status) = self.power.sample();
        let sensors = self.sensors.get();
        let networks = self
            .network
            .update(&read_text("/proc/net/dev"), &self.links.get(), elapsed);
        let mut storage = (*self.storage.get()).clone();
        let rates = self
            .disk_rates
            .update(&read_text("/proc/diskstats"), elapsed);
        storage::rates::overlay(&mut storage, &rates);
        let mut sockets = sockstat::Sockets::default();
        sockstat::add(&mut sockets, &read_text("/proc/net/sockstat"));
        sockstat::add(&mut sockets, &read_text("/proc/net/sockstat6"));
        let processes = collect_processes(&self.system, &gpu.gpus, &self.extras.get(), elapsed);
        let (memory, swap) = self.read_memory(elapsed);
        let mut snapshot = Snapshot {
            uptime_secs: System::uptime().ok(),
            load: System::load_average()
                .ok()
                .map(|load| [load.one, load.five, load.fifteen]),
            cores: self.system.cpus().iter().map(sysinfo::Cpu::usage).collect(),
            ram_used: memory.used,
            ram_total: memory.total,
            swap_used: swap.used,
            swap_total: swap.total,
            cpu: self.read_cpu(elapsed, cpu_watts),
            memory,
            swap,
            pressure: PressureView {
                cpu: pressure::parse(&read_text("/proc/pressure/cpu")),
                memory: pressure::parse(&read_text("/proc/pressure/memory")),
                io: pressure::parse(&read_text("/proc/pressure/io")),
            },
            gpu_driver: gpu.driver,
            gpu_cuda: gpu.cuda,
            gpus: gpu.gpus,
            gpu_status: gpu.status,
            cpu_power_status,
            temperatures: sensors
                .readings
                .iter()
                .filter(|reading| reading.kind == "temp")
                .map(|reading| Temperature {
                    label: format!("{} {}", reading.chip, reading.label),
                    celsius: reading.value,
                })
                .collect(),
            sensors: (*sensors).clone(),
            disks: mounted_disks(&storage),
            storage,
            networks,
            sockets,
            processes,
            ..Snapshot::default()
        };
        snapshot.point = history_point(&snapshot, background::now_ms());
        snapshot.sample_ms = started.elapsed().as_secs_f64() * 1000.0;
        snapshot
    }
}

fn mounted_disks(storage: &StorageView) -> Vec<Disk> {
    storage
        .disks
        .iter()
        .flat_map(|disk| &disk.partitions)
        .filter_map(|partition| {
            let usage = partition.usage.as_ref()?;
            Some(Disk {
                mount: partition.mount.clone()?,
                total: usage.total,
                available: usage.available,
            })
        })
        .collect()
}

/// What of a snapshot enters the graphs' history.
fn history_point(snapshot: &Snapshot, at_ms: u64) -> Point {
    let primary = snapshot.gpus.first();
    let (net_rx, net_tx) = network::physical_rates(&snapshot.networks);
    let (disk_read, disk_write) = storage::rates::totals(&snapshot.storage);
    let some10 = |view: Option<pressure::Pressure>| view.map(|p| tenths(f64::from(p.some[0])));
    Point {
        at_ms,
        cpu: tenths(f64::from(snapshot.cpu.usage)),
        ram: tenths(percent(snapshot.memory.used, snapshot.memory.total)),
        swap: tenths(percent(snapshot.swap.used, snapshot.swap.total)),
        iowait: tenths(f64::from(snapshot.cpu.shares.iowait)),
        gpu: primary.and_then(|g| g.utilization).map(|v| v as f32),
        vram: primary.and_then(gpu_memory_percent),
        // All or nothing: a sum over the GPUs that answered would understate.
        gpu_watts: snapshot
            .gpus
            .iter()
            .map(|g| g.watts)
            .sum::<Option<f64>>()
            .filter(|_| !snapshot.gpus.is_empty())
            .map(tenths),
        cpu_watts: snapshot.cpu.watts.map(tenths),
        cpu_temp: snapshot.sensors.cpu_temp.map(|t| tenths(f64::from(t))),
        gpu_temp: primary.and_then(|g| g.temperature).map(|t| t as f32),
        psi_cpu: some10(snapshot.pressure.cpu),
        psi_mem: some10(snapshot.pressure.memory),
        psi_io: some10(snapshot.pressure.io),
        net_rx: net_rx as u64,
        net_tx: net_tx as u64,
        disk_read: disk_read as u64,
        disk_write: disk_write as u64,
        swap_in: snapshot.swap.in_bps as u64,
        swap_out: snapshot.swap.out_bps as u64,
        disks: snapshot
            .storage
            .disks
            .iter()
            .filter_map(|disk| {
                let io = disk.io?;
                Some((
                    disk.name.clone(),
                    io.read_bps as u64,
                    io.write_bps as u64,
                    tenths(io.busy_pct),
                ))
            })
            .collect(),
        // An idle interface that is down has nothing to plot.
        nets: snapshot
            .networks
            .iter()
            .filter(|n| n.state == "up" || n.rx_bps > 0.0 || n.tx_bps > 0.0)
            .take(MAX_HISTORY_NETS)
            .map(|n| (n.name.clone(), n.rx_bps as u64, n.tx_bps as u64))
            .collect(),
        gpus: snapshot
            .gpus
            .iter()
            .map(|g| {
                (
                    g.index,
                    g.utilization.map(|v| v as f32),
                    gpu_memory_percent(g),
                    g.watts.map(tenths),
                    g.temperature.map(|t| t as f32),
                )
            })
            .collect(),
    }
}

fn gpu_memory_percent(gpu: &Gpu) -> Option<f32> {
    Some(tenths(percent(gpu.memory_used?, gpu.memory_total?)))
}

/// `/proc` files never block on hardware. A missing one parses as empty.
fn read_text(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn refresh_processes(system: &mut System) {
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_cpu()
            .with_memory()
            .with_disk_usage(),
    );
}

fn percent(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        100.0 * used as f64 / total as f64
    }
}

/// The kernel's letter for a process state, for the page to translate.
fn state_letter(status: ProcessStatus) -> &'static str {
    match status {
        ProcessStatus::Run => "R",
        ProcessStatus::Sleep => "S",
        ProcessStatus::UninterruptibleDiskSleep => "D",
        ProcessStatus::Zombie => "Z",
        ProcessStatus::Stop | ProcessStatus::Tracing => "T",
        ProcessStatus::Idle => "I",
        _ => "?",
    }
}

fn collect_processes(
    system: &System,
    gpus: &[Gpu],
    extras: &BTreeMap<u32, Extra>,
    elapsed: f64,
) -> Vec<Process> {
    let mut processes: Vec<_> = system
        .processes()
        .values()
        .map(|p| {
            let io = p.disk_usage();
            let pid = p.pid().as_u32();
            let extra = extras.get(&pid);
            Process {
                pid,
                parent: p.parent().map(sysinfo::Pid::as_u32),
                name: p.name().to_string_lossy().into_owned(),
                state: state_letter(p.status()),
                cpu: p.cpu_usage(),
                ram: p.memory(),
                swap: extra.map(|extra| extra.swap),
                threads: extra.map(|extra| extra.threads),
                sockets: extra.and_then(|extra| extra.sockets),
                read_bps: io.read_bytes as f64 / elapsed,
                write_bps: io.written_bytes as f64 / elapsed,
                read_total: io.total_read_bytes,
                write_total: io.total_written_bytes,
                started_secs: p.start_time(),
                gpu: gpus
                    .iter()
                    .flat_map(|g| &g.processes)
                    .filter(|p| p.pid == pid)
                    .cloned()
                    .collect(),
            }
        })
        .collect();
    // Include GPU contexts whose /proc entry is hidden or just exited.
    for gpu in gpus {
        for p in &gpu.processes {
            if let Some(found) = processes.iter_mut().find(|v| v.pid == p.pid) {
                if !found.gpu.iter().any(|v| v.index == p.index) {
                    found.gpu.push(p.clone());
                }
            } else {
                processes.push(Process {
                    pid: p.pid,
                    name: "Prozess nicht lesbar".into(),
                    state: "?",
                    gpu: vec![p.clone()],
                    ..Process::default()
                });
            }
        }
    }
    processes.sort_by_key(|p| p.pid);
    processes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DiskIo, Network, PhysicalDisk};

    fn snapshot() -> Snapshot {
        let network = |name: &str, state: &str, physical, rx_bps| Network {
            name: name.into(),
            state: state.into(),
            physical,
            rx_bps,
            ..Network::default()
        };
        Snapshot {
            cpu: CpuDetail {
                usage: 12.345,
                watts: Some(65.04),
                ..CpuDetail::default()
            },
            memory: Memory {
                used: 25,
                total: 100,
                ..Memory::default()
            },
            swap: Swap {
                used: 1,
                total: 3,
                in_bps: 4096.9,
                ..Swap::default()
            },
            gpus: vec![Gpu {
                index: 0,
                utilization: Some(98),
                memory_used: Some(1),
                memory_total: Some(4),
                watts: Some(190.26),
                temperature: Some(66),
                ..Gpu::default()
            }],
            storage: StorageView {
                disks: vec![
                    PhysicalDisk {
                        name: "nvme1n1".into(),
                        io: Some(DiskIo {
                            read_bps: 2000.7,
                            write_bps: 100.2,
                            busy_pct: 12.34,
                            ..DiskIo::default()
                        }),
                        ..PhysicalDisk::default()
                    },
                    PhysicalDisk {
                        name: "new-disk".into(),
                        ..PhysicalDisk::default()
                    },
                ],
                ..StorageView::default()
            },
            networks: vec![
                network("enp5s0", "up", true, 1000.0),
                network("enp6s0", "down", true, 0.0),
                network("lo", "unknown", false, 7.0),
            ],
            ..Snapshot::default()
        }
    }

    #[test]
    fn a_history_point_rounds_and_keeps_only_what_a_graph_can_use() {
        let point = history_point(&snapshot(), 42);
        assert_eq!(point.at_ms, 42);
        assert!((point.cpu - 12.3).abs() < 0.001);
        assert!((point.ram - 25.0).abs() < 0.001);
        assert!((point.swap - 33.3).abs() < 0.001);
        assert_eq!(point.swap_in, 4096);
        assert_eq!(point.gpu, Some(98.0));
        assert_eq!(point.vram, Some(25.0));
        assert_eq!(point.gpu_watts, Some(190.3));
        assert_eq!(point.cpu_watts, Some(65.0));
        assert_eq!(point.net_rx, 1000, "only physical adapters count");
        assert_eq!(point.disk_read, 2000);
        assert_eq!(point.disks, [("nvme1n1".to_owned(), 2000, 100, 12.3)]);
        let nets: Vec<_> = point.nets.iter().map(|n| n.0.as_str()).collect();
        assert_eq!(
            nets,
            ["enp5s0", "lo"],
            "an idle interface that is down is left out"
        );
        assert_eq!(
            point.gpus,
            [(0, Some(98.0), Some(25.0), Some(190.3), Some(66.0))]
        );
    }

    #[test]
    fn gpu_power_is_unknown_when_a_card_does_not_say() {
        let mut snapshot = snapshot();
        snapshot.gpus.push(Gpu::default());
        assert_eq!(history_point(&snapshot, 0).gpu_watts, None);
        snapshot.gpus.clear();
        assert_eq!(history_point(&snapshot, 0).gpu_watts, None);
    }

    #[test]
    fn process_states_keep_the_kernels_letters() {
        assert_eq!(state_letter(ProcessStatus::Run), "R");
        assert_eq!(state_letter(ProcessStatus::UninterruptibleDiskSleep), "D");
        assert_eq!(state_letter(ProcessStatus::Zombie), "Z");
        assert_eq!(state_letter(ProcessStatus::Unknown(99)), "?");
    }
}
