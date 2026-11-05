//! Physical disks, their partitions and what is mounted on them, refreshed on
//! the storage thread. The sampler adds the live I/O rates, which need only
//! `/proc/diskstats`, to the newest view it finds here.

mod inventory;
pub(crate) mod rates;
mod udev;
mod usage;

use crate::{
    background::now_ms,
    procfs::mountinfo::{self, Mount},
    types::{Partition, PhysicalDisk, StorageView, Usage, Volume},
};
use inventory::{DeviceNumber, RawDisk, RawPartition};
use std::{collections::BTreeMap, fs, path::Path, time::Instant};
use udev::Udev;
use usage::Growth;

/// In-memory filesystems that fill up and break jobs like a disk does.
const WATCHED_TMPFS: [&str; 3] = ["/tmp", "/dev/shm", "/run"];

#[derive(Default)]
pub(crate) struct Storage {
    growth: BTreeMap<String, Growth>,
    started: Option<Instant>,
}

impl Storage {
    pub fn refresh(&mut self) -> StorageView {
        let at_secs = self
            .started
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_secs_f64();
        let raw = inventory::scan(Path::new("/sys/block"));
        let (mounts, problem) = match fs::read_to_string("/proc/self/mountinfo") {
            Ok(text) => (mountinfo::parse(&text), None),
            Err(error) => (Vec::new(), Some(format!("Mounts nicht lesbar: {error}"))),
        };
        let growth = &mut self.growth;
        let read_usage = |path: &str| {
            let mut usage = usage::read(path)?;
            growth
                .entry(path.to_owned())
                .or_default()
                .apply(at_secs, &mut usage);
            Some(usage)
        };
        let (disks, volumes) = assemble(raw, &mounts, read_udev, read_usage);
        let seen = |mount: &str| {
            let shown = public_path(mount);
            disks
                .iter()
                .flat_map(|disk| &disk.partitions)
                .any(|partition| partition.mount.as_deref() == Some(&shown))
                || volumes.iter().any(|volume| volume.mount == shown)
        };
        self.growth.retain(|mount, _| seen(mount));
        StorageView {
            checked_ms: now_ms(),
            disks,
            volumes,
            problem,
        }
    }
}

fn read_udev(device: DeviceNumber) -> Udev {
    let path = format!("/run/udev/data/b{}:{}", device.0, device.1);
    fs::read_to_string(path)
        .map(|text| udev::parse(&text))
        .unwrap_or_default()
}

/// Joins the disks, the mount table and udev's names. `read_usage` is called
/// once per filesystem, with its real mount point.
fn assemble(
    raw: Vec<RawDisk>,
    mounts: &[Mount],
    udev: impl Fn(DeviceNumber) -> Udev,
    mut read_usage: impl FnMut(&str) -> Option<Usage>,
) -> (Vec<PhysicalDisk>, Vec<Volume>) {
    let mut mapped = Vec::new();
    let mut disks = Vec::with_capacity(raw.len());
    for disk in raw {
        // A filesystem straight on the disk has no partition: show the disk
        // itself as its one row, or it would have no mount point anywhere.
        let whole_disk = RawPartition {
            name: disk.name.clone(),
            device: disk.device,
            number: 0,
            size: disk.size,
            holders: disk.holders.clone(),
        };
        let whole_disk_used = disk.partitions.is_empty()
            && (udev(disk.device).fs_type.is_some() || !mounted_on(&whole_disk, mounts).is_empty());
        let rows = if whole_disk_used {
            std::slice::from_ref(&whole_disk)
        } else {
            &disk.partitions[..]
        };
        let partitions = rows
            .iter()
            .map(|raw| partition(raw, mounts, &udev, &mut read_usage, &mut mapped))
            .collect();
        disks.push(PhysicalDisk {
            model: disk.model,
            size: disk.size,
            transport: disk.transport.to_owned(),
            rotational: disk.rotational,
            removable: disk.removable,
            temperature: disk.temperature,
            link: disk.link,
            io: None,
            partitions,
            name: disk.name,
        });
    }
    let volumes = unmapped_volumes(mounts, &mapped, &mut read_usage);
    (disks, volumes)
}

fn partition(
    raw: &RawPartition,
    mounts: &[Mount],
    udev: &impl Fn(DeviceNumber) -> Udev,
    read_usage: &mut impl FnMut(&str) -> Option<Usage>,
    mapped: &mut Vec<DeviceNumber>,
) -> Partition {
    let here = mounted_on(raw, mounts);
    mapped.extend(here.iter().map(|mount| mount.device));
    let primary = here.first();
    let names = udev(raw.device);
    Partition {
        name: raw.name.clone(),
        label: names.label,
        part_name: names.part_name,
        // The kernel's driver name when mounted (`ntfs3`), else udev's guess.
        fs_type: primary.map(|mount| mount.fs_type.clone()).or(names.fs_type),
        mount: primary.map(|mount| public_path(&mount.path)),
        more_mounts: here.len().saturating_sub(1) as u32,
        read_only: primary.is_some_and(|mount| mount.read_only),
        size: raw.size,
        usage: primary.and_then(|mount| read_usage(&mount.path)),
        io: None,
    }
}

/// The mounts of a partition or of a device stacked on it, best first: the
/// filesystem's own root before bind mounts, then the shortest path.
fn mounted_on<'a>(partition: &RawPartition, mounts: &'a [Mount]) -> Vec<&'a Mount> {
    let mut found: Vec<_> = mounts
        .iter()
        .filter(|mount| {
            mount.device == partition.device || partition.holders.contains(&mount.device)
        })
        .collect();
    found.sort_by_key(|mount| (mount.root != "/", mount.path.len()));
    found
}

/// Filesystems on no listed partition still count: a full ZFS pool, an
/// unusual root device or a full `/tmp` break jobs just the same. Network
/// filesystems are left out because `statvfs` on a dead server hangs.
fn unmapped_volumes(
    mounts: &[Mount],
    mapped: &[DeviceNumber],
    read_usage: &mut impl FnMut(&str) -> Option<Usage>,
) -> Vec<Volume> {
    let mut seen = Vec::new();
    let mut volumes = Vec::new();
    for mount in mounts {
        if !is_local_volume(mount) || mapped.contains(&mount.device) || seen.contains(&mount.device)
        {
            continue;
        }
        seen.push(mount.device);
        volumes.push(Volume {
            mount: public_path(&mount.path),
            fs_type: mount.fs_type.clone(),
            read_only: mount.read_only,
            usage: read_usage(&mount.path),
        });
    }
    volumes.sort_by(|a, b| a.mount.cmp(&b.mount));
    volumes
}

fn is_local_volume(mount: &Mount) -> bool {
    match mount.fs_type.as_str() {
        "tmpfs" => WATCHED_TMPFS.contains(&mount.path.as_str()),
        "zfs" => true,
        // Snap and live-system images are read-only and always "full".
        "squashfs" | "iso9660" => false,
        _ => mount.source.starts_with("/dev/") && !mount.source.starts_with("/dev/loop"),
    }
}

/// A mount below a home directory would name its owner: the page has no
/// login, and `~` says the same.
fn public_path(path: &str) -> String {
    let Some(rest) = path.strip_prefix("/home/") else {
        return path.to_owned();
    };
    match rest.split_once('/') {
        Some((_, below)) => format!("~/{below}"),
        None => "~".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNTS: &str = "\
29 1 259:7 / / rw,relatime shared:1 - ext4 /dev/nvme1n1p2 rw
30 29 259:6 / /boot/efi rw,relatime shared:2 - vfat /dev/nvme1n1p1 rw
31 29 259:2 / /mnt/c ro,relatime shared:3 - ntfs3 /dev/nvme0n1p2 ro
32 29 259:9 / /mnt/xdata rw,noatime shared:4 - ext4 /dev/nvme2n1p1 rw
33 29 259:9 /docker /var/lib/docker rw,noatime shared:4 - ext4 /dev/nvme2n1p1 rw
34 29 0:30 / /tmp rw,nosuid - tmpfs tmpfs rw
35 29 0:31 / /dev/shm rw,nosuid - tmpfs tmpfs rw
36 29 0:32 / /run/user/1000 rw,nosuid - tmpfs tmpfs rw
37 29 0:25 / /sys rw,nosuid - sysfs sysfs rw
38 29 7:0 / /snap/core/1 ro - squashfs /dev/loop0 ro
39 29 0:40 / /mnt/nas rw - nfs4 nas:/share rw
40 29 0:41 / /tank rw - zfs tank rw
";

    fn raw_disks() -> Vec<RawDisk> {
        let part = |name: &str, minor, number, size| RawPartition {
            name: name.into(),
            device: (259, minor),
            number,
            size,
            holders: Vec::new(),
        };
        vec![
            RawDisk {
                name: "nvme0n1".into(),
                device: (259, 0),
                model: "Sabrent Rocket Q4".into(),
                size: 2000,
                transport: "NVMe",
                partitions: vec![part("nvme0n1p1", 1, 1, 16), part("nvme0n1p2", 2, 2, 1900)],
                ..RawDisk::default()
            },
            RawDisk {
                name: "nvme1n1".into(),
                device: (259, 5),
                model: "Samsung SSD 990 PRO 2TB".into(),
                size: 2000,
                transport: "NVMe",
                partitions: vec![part("nvme1n1p1", 6, 1, 1), part("nvme1n1p2", 7, 2, 1999)],
                ..RawDisk::default()
            },
            RawDisk {
                name: "nvme2n1".into(),
                device: (259, 8),
                model: "Lexar SSD NM710 2TB".into(),
                size: 2000,
                transport: "NVMe",
                partitions: vec![part("nvme2n1p1", 9, 1, 2000)],
                ..RawDisk::default()
            },
        ]
    }

    fn names(device: DeviceNumber) -> Udev {
        match device {
            (259, 1) => Udev {
                fs_type: Some("ext4".into()),
                label: Some("Games".into()),
                part_name: Some("Microsoft reserved partition".into()),
            },
            (259, 2) => Udev {
                fs_type: Some("ntfs".into()),
                part_name: Some("Basic data partition".into()),
                ..Udev::default()
            },
            (259, 9) => Udev {
                fs_type: Some("ext4".into()),
                label: Some("xdata".into()),
                part_name: Some("xdata".into()),
            },
            _ => Udev::default(),
        }
    }

    #[allow(clippy::unnecessary_wraps)] // It stands in for `statvfs`, which can fail.
    fn fake_usage(_mount: &str) -> Option<Usage> {
        Some(Usage {
            total: 100,
            used: 40,
            available: 60,
            used_pct: 40.0,
            ..Usage::default()
        })
    }

    #[test]
    fn mounts_land_on_their_partitions_by_device_number() {
        let mounts = mountinfo::parse(MOUNTS);
        let (disks, _) = assemble(raw_disks(), &mounts, names, fake_usage);
        assert_eq!(disks.len(), 3);

        let sabrent = &disks[0];
        assert_eq!(sabrent.model, "Sabrent Rocket Q4");
        let reserved = &sabrent.partitions[0];
        assert_eq!(reserved.name, "nvme0n1p1");
        assert_eq!(reserved.label.as_deref(), Some("Games"));
        assert_eq!(reserved.fs_type.as_deref(), Some("ext4"));
        assert_eq!(reserved.mount, None);
        assert_eq!(reserved.usage, None, "not mounted: nothing to measure");

        let windows = &sabrent.partitions[1];
        assert_eq!(windows.mount.as_deref(), Some("/mnt/c"));
        assert_eq!(
            windows.fs_type.as_deref(),
            Some("ntfs3"),
            "the mount's driver"
        );
        assert_eq!(windows.part_name.as_deref(), Some("Basic data partition"));
        assert!(windows.read_only);
        assert_eq!(windows.usage.as_ref().unwrap().used, 40);

        let system = &disks[1].partitions[1];
        assert_eq!(system.mount.as_deref(), Some("/"));
        assert!(!system.read_only);
        assert_eq!(disks[1].partitions[0].mount.as_deref(), Some("/boot/efi"));
    }

    #[test]
    fn a_bind_mount_does_not_replace_the_filesystem_root() {
        let mounts = mountinfo::parse(MOUNTS);
        let (disks, _) = assemble(raw_disks(), &mounts, names, fake_usage);
        let xdata = &disks[2].partitions[0];
        assert_eq!(xdata.mount.as_deref(), Some("/mnt/xdata"));
        assert_eq!(xdata.more_mounts, 1);
        assert_eq!(xdata.label.as_deref(), Some("xdata"));
    }

    #[test]
    fn a_mount_on_a_stacked_device_belongs_to_the_partition_below() {
        let mut raw = raw_disks();
        raw[1].partitions[1].holders = vec![(253, 0)];
        let mounts = mountinfo::parse(
            "1 0 253:0 / / rw - ext4 /dev/mapper/cryptroot rw\n\
             2 1 259:6 / /boot/efi rw - vfat /dev/nvme1n1p1 rw\n",
        );
        let (disks, volumes) = assemble(raw, &mounts, names, fake_usage);
        assert_eq!(disks[1].partitions[1].mount.as_deref(), Some("/"));
        assert!(volumes.is_empty(), "it is mapped, not a loose volume");
    }

    #[test]
    fn a_filesystem_on_the_bare_disk_gets_its_own_row() {
        let raw = vec![RawDisk {
            name: "sdb".into(),
            device: (8, 16),
            size: 500,
            transport: "USB",
            ..RawDisk::default()
        }];
        let mounts = mountinfo::parse("1 0 8:16 / /mnt/usb rw - exfat /dev/sdb rw\n");
        let (disks, _) = assemble(raw, &mounts, names, fake_usage);
        assert_eq!(disks[0].partitions.len(), 1);
        assert_eq!(disks[0].partitions[0].name, "sdb");
        assert_eq!(disks[0].partitions[0].mount.as_deref(), Some("/mnt/usb"));
    }

    #[test]
    fn a_blank_disk_has_no_rows() {
        let raw = vec![RawDisk {
            name: "sdc".into(),
            device: (8, 32),
            size: 500,
            ..RawDisk::default()
        }];
        let (disks, _) = assemble(raw, &[], names, fake_usage);
        assert!(disks[0].partitions.is_empty());
    }

    #[test]
    fn loose_volumes_are_local_and_watched_only() {
        let mounts = mountinfo::parse(MOUNTS);
        let mut asked = Vec::new();
        let (_, volumes) = assemble(raw_disks(), &mounts, names, |mount: &str| {
            asked.push(mount.to_owned());
            fake_usage(mount)
        });
        let shown: Vec<_> = volumes.iter().map(|v| v.mount.as_str()).collect();
        assert_eq!(shown, ["/dev/shm", "/tank", "/tmp"]);
        assert!(
            !asked.iter().any(|mount| mount == "/mnt/nas"),
            "statvfs on a network mount can hang: {asked:?}"
        );
    }

    #[test]
    fn a_home_directory_mount_does_not_name_its_owner() {
        assert_eq!(public_path("/home/ace/data"), "~/data");
        assert_eq!(public_path("/home/ace"), "~");
        assert_eq!(public_path("/mnt/xdata"), "/mnt/xdata");
        let raw = vec![RawDisk {
            name: "sdb".into(),
            device: (8, 16),
            size: 500,
            ..RawDisk::default()
        }];
        let mounts = mountinfo::parse("1 0 8:16 / /home/ace/usb rw - exfat /dev/sdb rw\n");
        let mut asked = Vec::new();
        let (disks, _) = assemble(raw, &mounts, names, |mount: &str| {
            asked.push(mount.to_owned());
            fake_usage(mount)
        });
        assert_eq!(disks[0].partitions[0].mount.as_deref(), Some("~/usb"));
        assert_eq!(asked, ["/home/ace/usb"], "measured at its real path");
    }
}
