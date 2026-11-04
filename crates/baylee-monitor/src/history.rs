//! The graphs' memory: every sample of the last hour, and one averaged point
//! per minute for the last day, which is what a long training run is judged
//! by. Everything stays in RAM and resets with the service.

use crate::types::{Point, tenths};
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};

/// A day of minutes.
const MINUTES: usize = 24 * 60;
const MINUTE_MS: u64 = 60_000;

pub(crate) struct History {
    samples: VecDeque<Point>,
    keep: usize,
    minutes: VecDeque<Point>,
    open: Minute,
}

/// What a new viewer receives: both rings, serialized straight from memory.
#[derive(Serialize)]
pub(crate) struct Recorded<'a> {
    pub history: &'a VecDeque<Point>,
    pub minutes: &'a VecDeque<Point>,
}

impl History {
    pub fn new(keep: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(keep),
            keep,
            minutes: VecDeque::with_capacity(MINUTES),
            open: Minute::default(),
        }
    }

    pub fn push(&mut self, point: Point) {
        let index = point.at_ms / MINUTE_MS;
        if index != self.open.index {
            if let Some(finished) = self.open.finish() {
                if self.minutes.len() == MINUTES {
                    self.minutes.pop_front();
                }
                self.minutes.push_back(finished);
            }
            self.open = Minute {
                index,
                ..Minute::default()
            };
        }
        self.open.add(&point);
        if self.samples.len() == self.keep {
            self.samples.pop_front();
        }
        self.samples.push_back(point);
    }

    pub fn recorded(&self) -> Recorded<'_> {
        Recorded {
            history: &self.samples,
            minutes: &self.minutes,
        }
    }
}

/// The mean of what was available; nothing available stays nothing, so a gap
/// in a sensor stays a gap and is not averaged into a zero.
#[derive(Clone, Copy, Default)]
struct Mean {
    sum: f64,
    count: u32,
}

impl Mean {
    fn add(&mut self, value: Option<f64>) {
        if let Some(value) = value {
            self.sum += value;
            self.count += 1;
        }
    }

    fn get(&self) -> Option<f64> {
        (self.count > 0).then(|| self.sum / f64::from(self.count))
    }

    /// A rate or percentage that is present in every point of its minute.
    fn value(&self) -> f64 {
        self.get().unwrap_or(0.0)
    }
}

const NUMBERS: usize = 19;

/// Every plain number of a point, in the order `Minute::finish` unpacks them.
#[allow(clippy::cast_precision_loss)] // Rates.
fn numbers(point: &Point) -> [Option<f64>; NUMBERS] {
    let optional = |value: Option<f32>| value.map(f64::from);
    [
        Some(f64::from(point.cpu)),
        Some(f64::from(point.ram)),
        Some(f64::from(point.swap)),
        Some(f64::from(point.iowait)),
        optional(point.gpu),
        optional(point.vram),
        optional(point.gpu_watts),
        optional(point.cpu_watts),
        optional(point.cpu_temp),
        optional(point.gpu_temp),
        optional(point.psi_cpu),
        optional(point.psi_mem),
        optional(point.psi_io),
        Some(point.net_rx as f64),
        Some(point.net_tx as f64),
        Some(point.disk_read as f64),
        Some(point.disk_write as f64),
        Some(point.swap_in as f64),
        Some(point.swap_out as f64),
    ]
}

/// The minute being collected.
#[derive(Default)]
struct Minute {
    index: u64,
    numbers: [Mean; NUMBERS],
    disks: BTreeMap<String, [Mean; 3]>,
    nets: BTreeMap<String, [Mean; 2]>,
    gpus: BTreeMap<u32, [Mean; 4]>,
}

impl Minute {
    #[allow(clippy::cast_precision_loss)] // Rates.
    fn add(&mut self, point: &Point) {
        for (mean, value) in self.numbers.iter_mut().zip(numbers(point)) {
            mean.add(value);
        }
        for (name, read, write, busy) in &point.disks {
            let means = self.disks.entry(name.clone()).or_default();
            means[0].add(Some(*read as f64));
            means[1].add(Some(*write as f64));
            means[2].add(Some(f64::from(*busy)));
        }
        for (name, rx, tx) in &point.nets {
            let means = self.nets.entry(name.clone()).or_default();
            means[0].add(Some(*rx as f64));
            means[1].add(Some(*tx as f64));
        }
        for (index, utilization, vram, watts, temperature) in &point.gpus {
            let means = self.gpus.entry(*index).or_default();
            means[0].add(utilization.map(f64::from));
            means[1].add(vram.map(f64::from));
            means[2].add(watts.map(f64::from));
            means[3].add(temperature.map(f64::from));
        }
    }

    /// The minute's averages, stamped at its middle; `None` if it saw nothing.
    fn finish(&self) -> Option<Point> {
        self.numbers[0].get()?;
        let [
            cpu,
            ram,
            swap,
            iowait,
            gpu,
            vram,
            gpu_watts,
            cpu_watts,
            cpu_temp,
            gpu_temp,
            psi_cpu,
            psi_mem,
            psi_io,
            net_rx,
            net_tx,
            disk_read,
            disk_write,
            swap_in,
            swap_out,
        ] = self.numbers;
        let optional = |mean: Mean| mean.get().map(tenths);
        Some(Point {
            at_ms: self.index * MINUTE_MS + MINUTE_MS / 2,
            cpu: tenths(cpu.value()),
            ram: tenths(ram.value()),
            swap: tenths(swap.value()),
            iowait: tenths(iowait.value()),
            gpu: optional(gpu),
            vram: optional(vram),
            gpu_watts: optional(gpu_watts),
            cpu_watts: optional(cpu_watts),
            cpu_temp: optional(cpu_temp),
            gpu_temp: optional(gpu_temp),
            psi_cpu: optional(psi_cpu),
            psi_mem: optional(psi_mem),
            psi_io: optional(psi_io),
            net_rx: net_rx.value() as u64,
            net_tx: net_tx.value() as u64,
            disk_read: disk_read.value() as u64,
            disk_write: disk_write.value() as u64,
            swap_in: swap_in.value() as u64,
            swap_out: swap_out.value() as u64,
            disks: self
                .disks
                .iter()
                .map(|(name, [read, write, busy])| {
                    (
                        name.clone(),
                        read.value() as u64,
                        write.value() as u64,
                        tenths(busy.value()),
                    )
                })
                .collect(),
            nets: self
                .nets
                .iter()
                .map(|(name, [rx, tx])| (name.clone(), rx.value() as u64, tx.value() as u64))
                .collect(),
            gpus: self
                .gpus
                .iter()
                .map(|(index, [utilization, vram, watts, temperature])| {
                    (
                        *index,
                        optional(*utilization),
                        optional(*vram),
                        optional(*watts),
                        optional(*temperature),
                    )
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(at_ms: u64, cpu: f32) -> Point {
        Point {
            at_ms,
            cpu,
            ..Point::default()
        }
    }

    #[test]
    fn samples_stay_bounded_and_keep_the_newest() {
        let mut history = History::new(3);
        for at in 0..5 {
            history.push(point(at * 1000, 1.0));
        }
        let recorded = history.recorded();
        let times: Vec<_> = recorded.history.iter().map(|p| p.at_ms).collect();
        assert_eq!(times, [2000, 3000, 4000]);
    }

    #[test]
    fn a_finished_minute_becomes_one_averaged_point() {
        let mut history = History::new(10);
        history.push(Point {
            net_rx: 100,
            gpu: Some(10.0),
            disks: vec![("nvme0n1".into(), 1000, 0, 10.0)],
            nets: vec![("enp5s0".into(), 10, 30)],
            gpus: vec![(0, Some(50.0), None, Some(100.0), None)],
            ..point(60_000, 20.0)
        });
        history.push(Point {
            net_rx: 300,
            disks: vec![("nvme0n1".into(), 3000, 0, 30.0)],
            nets: vec![("enp5s0".into(), 30, 50)],
            gpus: vec![(0, None, None, Some(200.0), None)],
            ..point(90_000, 40.0)
        });
        assert!(history.recorded().minutes.is_empty(), "the minute is open");
        history.push(point(120_000, 99.0));
        let minutes = history.recorded().minutes;
        assert_eq!(minutes.len(), 1);
        let minute = &minutes[0];
        assert_eq!(minute.at_ms, 90_000, "stamped at the minute's middle");
        assert!((minute.cpu - 30.0).abs() < 0.01);
        assert_eq!(minute.net_rx, 200);
        assert_eq!(minute.gpu, Some(10.0), "a gap does not count as zero");
        assert_eq!(minute.vram, None, "never measured stays unknown");
        assert_eq!(minute.disks, [("nvme0n1".to_owned(), 2000, 0, 20.0)]);
        assert_eq!(minute.nets, [("enp5s0".to_owned(), 20, 40)]);
        assert_eq!(minute.gpus, [(0, Some(50.0), None, Some(150.0), None)]);
    }

    #[test]
    fn a_device_that_vanishes_ends_its_series() {
        let mut history = History::new(10);
        history.push(Point {
            nets: vec![("veth1".into(), 10, 10)],
            ..point(60_000, 1.0)
        });
        history.push(point(120_000, 1.0));
        history.push(point(180_000, 1.0));
        let minutes = history.recorded().minutes;
        assert_eq!(minutes.len(), 2);
        assert_eq!(minutes[0].nets.len(), 1);
        assert!(minutes[1].nets.is_empty());
    }

    /// Every new viewer gets all of this at once, on a phone too, and the
    /// service must stay far below its 192 MiB limit with twelve viewers.
    #[test]
    fn a_full_history_stays_a_few_megabytes() {
        let mut history = History::new(3600);
        for second in 0..(24 * 3600) {
            let busy = 1_000_000_000 + second;
            history.push(Point {
                at_ms: 1_793_000_000_000 + second * 1000,
                cpu: 12.3,
                ram: 34.5,
                swap: 12.3,
                iowait: 1.2,
                gpu: Some(98.0),
                vram: Some(65.5),
                gpu_watts: Some(190.3),
                cpu_temp: Some(56.4),
                gpu_temp: Some(66.0),
                psi_cpu: Some(1.2),
                psi_mem: Some(0.1),
                psi_io: Some(11.3),
                net_rx: busy,
                net_tx: busy,
                disk_read: busy,
                disk_write: busy,
                disks: ["nvme0n1", "nvme1n1", "nvme2n1"]
                    .map(|name| (name.to_owned(), busy, busy, 12.3))
                    .to_vec(),
                nets: ["enp5s0", "lo"]
                    .map(|name| (name.to_owned(), busy, busy))
                    .to_vec(),
                gpus: vec![(0, Some(98.0), Some(65.5), Some(190.3), Some(66.0))],
                ..Point::default()
            });
        }
        let json = serde_json::to_string(&history.recorded()).unwrap();
        assert!(json.len() < 4 << 20, "{} bytes", json.len());
    }

    #[test]
    fn the_minute_ring_stops_at_a_day() {
        let mut history = History::new(1);
        for minute in 0..(MINUTES as u64 + 10) {
            history.push(point(minute * MINUTE_MS, 1.0));
        }
        assert_eq!(history.recorded().minutes.len(), MINUTES);
    }
}
