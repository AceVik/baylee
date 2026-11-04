//! One shared, blocking sampler. Browser count never changes collection work.
#![allow(clippy::cast_precision_loss)] // Percentages/rates, not exact byte accounting.

use crate::{
    background::{self, Latest},
    gpu::Gpus,
    network::{self, Link},
    power::Power,
    procfs::{
        meminfo, per_second, pressure, sockstat,
        stat::{self, Stat},
        swaps,
        vmstat::{self, Vmstat},
    },
    sensors::{self, SensorView},
    storage::{self, Storage},
    types::{
        CpuDetail, Disk, Facts, GpuView, Memory, Point, PressureView, Process, Snapshot,
        StorageView, Swap, Temperature,
    },
};
use anyhow::Context;
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Duration, Instant},
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// Link speed and state change rarely, and asking the driver can be slow.
const LINKS_EVERY: Duration = Duration::from_secs(5);
/// Temperatures and fans move over seconds; an embedded controller is slow.
const SENSORS_EVERY: Duration = Duration::from_secs(2);
/// Often enough that a filling disk shows within seconds; `statvfs` and the
/// sysfs walk are its own thread's business, so a sick disk stalls only this.
const STORAGE_EVERY: Duration = Duration::from_secs(5);

pub(crate) struct Sampler {
    system: System,
    network: network::Tracker,
    links: Latest<BTreeMap<String, Link>>,
    sensors: Latest<SensorView>,
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

    fn read_cpu(&mut self, elapsed: f64) -> CpuDetail {
        let now = stat::parse(&read_text("/proc/stat"));
        let before = std::mem::replace(&mut self.stat, now);
        CpuDetail {
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
        let links = self.links.get();
        let sensors = self.sensors.get();
        let networks = self
            .network
            .update(&read_text("/proc/net/dev"), &links, elapsed);
        let (net_rx, net_tx) = network::physical_rates(&networks);
        let mut storage = (*self.storage.get()).clone();
        let rates = self
            .disk_rates
            .update(&read_text("/proc/diskstats"), elapsed);
        storage::rates::overlay(&mut storage, &rates);
        let (disk_read, disk_write) = storage::rates::totals(&storage);
        let mut sockets = sockstat::Sockets::default();
        sockstat::add(&mut sockets, &read_text("/proc/net/sockstat"));
        sockstat::add(&mut sockets, &read_text("/proc/net/sockstat6"));
        let processes = collect_processes(&self.system, &gpu.gpus, elapsed);
        let primary = gpu.gpus.first();
        let load = System::load_average()
            .ok()
            .map(|load| [load.one, load.five, load.fifteen]);
        let (memory, swap) = self.read_memory(elapsed);
        let cpu = self.read_cpu(elapsed);
        let pressure = PressureView {
            cpu: pressure::parse(&read_text("/proc/pressure/cpu")),
            memory: pressure::parse(&read_text("/proc/pressure/memory")),
            io: pressure::parse(&read_text("/proc/pressure/io")),
        };
        let some10 = |view: Option<pressure::Pressure>| view.map(|p| p.some[0]);
        Snapshot {
            point: Point {
                at_ms: background::now_ms(),
                cpu: self.system.global_cpu_usage(),
                ram: percent(memory.used, memory.total),
                swap: percent(swap.used, swap.total),
                swap_in: swap.in_bps as u64,
                swap_out: swap.out_bps as u64,
                iowait: cpu.shares.iowait,
                disk_read: disk_read as u64,
                disk_write: disk_write as u64,
                cpu_temp: sensors.cpu_temp,
                gpu_temp: primary.and_then(|g| g.temperature).map(|t| t as f32),
                psi_cpu: some10(pressure.cpu),
                psi_mem: some10(pressure.memory),
                psi_io: some10(pressure.io),
                gpu: primary.and_then(|g| g.utilization).map(f64::from),
                vram: primary.and_then(|g| Some(percent(g.memory_used?, g.memory_total?))),
                gpu_watts: gpu
                    .gpus
                    .iter()
                    .map(|g| g.watts)
                    .collect::<Option<Vec<_>>>()
                    .filter(|v| !v.is_empty())
                    .map(|v| v.iter().sum()),
                cpu_watts,
                net_rx,
                net_tx,
            },
            uptime_secs: System::uptime().ok(),
            load,
            cores: self.system.cpus().iter().map(sysinfo::Cpu::usage).collect(),
            ram_used: memory.used,
            ram_total: memory.total,
            swap_used: swap.used,
            swap_total: swap.total,
            cpu,
            memory,
            swap,
            pressure,
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
            sample_ms: started.elapsed().as_secs_f64() * 1000.0,
        }
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

fn collect_processes(system: &System, gpus: &[crate::types::Gpu], elapsed: f64) -> Vec<Process> {
    let mut processes: Vec<_> = system
        .processes()
        .values()
        .map(|p| {
            let io = p.disk_usage();
            let pid = p.pid().as_u32();
            Process {
                pid,
                name: p.name().to_string_lossy().into_owned(),
                cpu: p.cpu_usage(),
                ram: p.memory(),
                read_bps: io.read_bytes as f64 / elapsed,
                write_bps: io.written_bytes as f64 / elapsed,
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
                    cpu: 0.0,
                    ram: 0,
                    read_bps: 0.0,
                    write_bps: 0.0,
                    gpu: vec![p.clone()],
                });
            }
        }
    }
    processes.sort_by_key(|p| p.pid);
    processes
}
