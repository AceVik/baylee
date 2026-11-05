//! Live read and write rates from `/proc/diskstats`, at the sampler's pace.
//! The file is plain kernel memory: reading it never touches a disk.

use crate::{
    procfs::{
        diskstats::{self, Counters, SECTOR},
        per_second,
    },
    types::{DiskIo, StorageView},
};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Tracker {
    previous: BTreeMap<String, Counters>,
}

impl Tracker {
    /// Rates since the last call, by device name. A device seen for the first
    /// time has no rate yet.
    pub fn update(&mut self, text: &str, elapsed_secs: f64) -> BTreeMap<String, DiskIo> {
        let now: BTreeMap<_, _> = diskstats::parse(text).into_iter().collect();
        let rates = now
            .iter()
            .filter_map(|(name, counters)| {
                let before = self.previous.get(name)?;
                Some((name.clone(), rate(before, counters, elapsed_secs)))
            })
            .collect();
        self.previous = now;
        rates
    }
}

#[allow(clippy::cast_precision_loss)] // Sector size times a rate, not exact bytes.
fn rate(before: &Counters, now: &Counters, elapsed: f64) -> DiskIo {
    let sector = SECTOR as f64;
    DiskIo {
        read_bps: per_second(before.read_sectors, now.read_sectors, elapsed) * sector,
        write_bps: per_second(before.write_sectors, now.write_sectors, elapsed) * sector,
        read_iops: per_second(before.reads, now.reads, elapsed),
        write_iops: per_second(before.writes, now.writes, elapsed),
        // Milliseconds busy per second, as a percentage.
        busy_pct: (per_second(before.busy_ms, now.busy_ms, elapsed) / 10.0).min(100.0),
        in_flight: now.in_flight,
    }
}

/// Puts each device's rates on the newest inventory.
pub(crate) fn overlay(view: &mut StorageView, rates: &BTreeMap<String, DiskIo>) {
    for disk in &mut view.disks {
        disk.io = rates.get(&disk.name).copied();
        for partition in &mut disk.partitions {
            partition.io = rates.get(&partition.name).copied();
        }
    }
}

/// The machine's total, over disks only: partitions would count twice.
pub(crate) fn totals(view: &StorageView) -> (f64, f64) {
    view.disks
        .iter()
        .filter_map(|disk| disk.io)
        .fold((0.0, 0.0), |(read, write), io| {
            (read + io.read_bps, write + io.write_bps)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Partition, PhysicalDisk};

    fn line(name: &str, reads: u64, read_sectors: u64, writes: u64, busy_ms: u64) -> String {
        format!(
            " 259 0 {name} {reads} 0 {read_sectors} 0 {writes} 0 {} 0 3 {busy_ms} 0\n",
            writes * 8
        )
    }

    #[test]
    fn rates_are_bytes_iops_and_busy_time_per_second() {
        let mut tracker = Tracker::default();
        assert!(
            tracker
                .update(&line("nvme0n1", 10, 100, 5, 1000), 1.0)
                .is_empty(),
            "a first reading has nothing to compare with"
        );
        let rates = tracker.update(&line("nvme0n1", 30, 2148, 25, 1500), 2.0);
        let io = rates["nvme0n1"];
        assert!((io.read_bps - 1024.0 * 512.0).abs() < f64::EPSILON);
        assert!((io.write_bps - 80.0 * 512.0).abs() < f64::EPSILON);
        assert!((io.read_iops - 10.0).abs() < f64::EPSILON);
        assert!((io.write_iops - 10.0).abs() < f64::EPSILON);
        assert!((io.busy_pct - 25.0).abs() < f64::EPSILON);
        assert_eq!(io.in_flight, 3);
    }

    #[test]
    fn busy_time_never_passes_a_hundred_percent() {
        let mut tracker = Tracker::default();
        tracker.update(&line("sda", 0, 0, 0, 0), 1.0);
        let rates = tracker.update(&line("sda", 0, 0, 0, 1500), 1.0);
        assert!((rates["sda"].busy_pct - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rates_land_on_disks_and_partitions_and_total_only_the_disks() {
        let io = |read_bps| DiskIo {
            read_bps,
            ..DiskIo::default()
        };
        let rates = BTreeMap::from([
            ("nvme1n1".to_owned(), io(100.0)),
            ("nvme1n1p2".to_owned(), io(100.0)),
            ("nvme2n1".to_owned(), io(50.0)),
        ]);
        let disk = |name: &str, partition: &str| PhysicalDisk {
            name: name.into(),
            partitions: vec![Partition {
                name: partition.into(),
                ..Partition::default()
            }],
            ..PhysicalDisk::default()
        };
        let mut view = StorageView {
            disks: vec![disk("nvme1n1", "nvme1n1p2"), disk("nvme2n1", "nvme2n1p1")],
            ..StorageView::default()
        };
        overlay(&mut view, &rates);
        assert_eq!(view.disks[0].partitions[0].io, Some(io(100.0)));
        assert_eq!(view.disks[1].partitions[0].io, None, "not in diskstats");
        let (read, write) = totals(&view);
        assert!((read - 150.0).abs() < f64::EPSILON);
        assert!(write.abs() < f64::EPSILON);
    }
}
