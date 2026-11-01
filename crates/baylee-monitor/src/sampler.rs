//! One shared, blocking sampler. Browser count never changes collection work.
#![allow(clippy::cast_precision_loss)] // Percentages/rates, not exact byte accounting.

use crate::{
    gpu::Gpus,
    power::Power,
    types::{Disk, Facts, Network, Point, Process, Snapshot, Temperature},
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{Components, Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};

pub(crate) struct Sampler {
    system: System,
    networks: Networks,
    components: Components,
    disks: Disks,
    gpu: Gpus,
    power: Power,
    previous: Instant,
    disk_at: Instant,
}

impl Sampler {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_all();
        system.refresh_memory();
        refresh_processes(&mut system);
        Self {
            system,
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            gpu: Gpus::new(),
            power: Power::new(),
            previous: Instant::now(),
            disk_at: Instant::now(),
        }
    }

    pub fn facts(&self, interval_ms: u64) -> Facts {
        Facts {
            hostname: System::host_name().unwrap_or_else(|| "Workstation".into()),
            os: System::long_os_version().unwrap_or_else(|| std::env::consts::OS.into()),
            cpu: self
                .system
                .cpus()
                .first()
                .map_or("CPU", sysinfo::Cpu::brand)
                .into(),
            physical_cores: System::physical_core_count(),
            threads: self.system.cpus().len(),
            interval_ms,
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
        self.system.refresh_memory();
        refresh_processes(&mut self.system);
        self.networks.refresh(true);
        self.components.refresh(true);
        // statvfs is slower, and free space does not need 1 Hz updates.
        if started.duration_since(self.disk_at).as_secs() >= 15 {
            self.disks.refresh(true);
            self.disk_at = started;
        }
        let (gpus, gpu_status) = self.gpu.sample();
        let (cpu_watts, cpu_power_status) = self.power.sample();
        let networks: Vec<_> = self
            .networks
            .iter()
            .filter(|(name, _)| *name != "lo")
            .map(|(name, data)| Network {
                name: name.clone(),
                rx_bps: data.received() as f64 / elapsed,
                tx_bps: data.transmitted() as f64 / elapsed,
            })
            .collect();
        // Aggregate physical interfaces only; bridges/veth repeat the same traffic.
        let physical: Vec<_> = networks
            .iter()
            .filter(|n| {
                std::path::Path::new("/sys/class/net")
                    .join(&n.name)
                    .join("device")
                    .exists()
            })
            .collect();
        let processes = collect_processes(&self.system, &gpus, elapsed);
        let primary = gpus.first();
        let load = System::load_average();
        Snapshot {
            point: Point {
                at_ms: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                cpu: self.system.global_cpu_usage(),
                ram: percent(self.system.used_memory(), self.system.total_memory()),
                gpu: primary.and_then(|g| g.utilization).map(f64::from),
                vram: primary.and_then(|g| Some(percent(g.memory_used?, g.memory_total?))),
                gpu_watts: gpus
                    .iter()
                    .map(|g| g.watts)
                    .collect::<Option<Vec<_>>>()
                    .filter(|v| !v.is_empty())
                    .map(|v| v.iter().sum()),
                cpu_watts,
                net_rx: physical.iter().map(|n| n.rx_bps).sum(),
                net_tx: physical.iter().map(|n| n.tx_bps).sum(),
            },
            uptime_secs: System::uptime(),
            load: [load.one, load.five, load.fifteen],
            cores: self
                .system
                .cpus()
                .iter()
                .map(sysinfo::Cpu::cpu_usage)
                .collect(),
            ram_used: self.system.used_memory(),
            ram_total: self.system.total_memory(),
            swap_used: self.system.used_swap(),
            swap_total: self.system.total_swap(),
            gpus,
            gpu_status,
            cpu_power_status,
            temperatures: self
                .components
                .iter()
                .filter_map(|c| {
                    Some(Temperature {
                        label: c.label().into(),
                        celsius: c
                            .temperature()
                            .filter(|v| v.is_finite() && *v > -20.0 && *v < 150.0)?,
                    })
                })
                .collect(),
            disks: self
                .disks
                .iter()
                .filter(|d| !d.is_removable())
                .map(|d| Disk {
                    mount: d.mount_point().to_string_lossy().into_owned(),
                    total: d.total_space(),
                    available: d.available_space(),
                })
                .collect(),
            networks,
            processes,
            sample_ms: started.elapsed().as_secs_f64() * 1000.0,
        }
    }
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
