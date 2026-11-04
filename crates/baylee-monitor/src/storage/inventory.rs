//! The physical disks in `/sys/block` and the partitions under each. Loop,
//! RAM and device-mapper devices have no `device` link and are not disks.

use crate::types::PcieLink;
use std::{fs, path::Path};

pub(super) type DeviceNumber = (u32, u32);

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct RawDisk {
    pub name: String,
    pub device: DeviceNumber,
    pub model: String,
    pub size: u64,
    pub transport: &'static str,
    pub rotational: bool,
    pub removable: bool,
    pub temperature: Option<f32>,
    pub link: Option<PcieLink>,
    /// Devices stacked on the whole disk, such as LVM without a partition table.
    pub holders: Vec<DeviceNumber>,
    pub partitions: Vec<RawPartition>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RawPartition {
    pub name: String,
    pub device: DeviceNumber,
    pub number: u32,
    pub size: u64,
    /// Devices stacked on this partition, such as LUKS or LVM.
    pub holders: Vec<DeviceNumber>,
}

/// `/sys/block/*/size` counts 512-byte sectors, whatever the device's.
const SECTOR: u64 = 512;

fn text(path: &Path) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn number<T: std::str::FromStr>(path: &Path) -> Option<T> {
    text(path)?.parse().ok()
}

fn device_number(path: &Path) -> Option<DeviceNumber> {
    let value = text(path)?;
    let (major, minor) = value.split_once(':')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

fn entries(path: &Path) -> Vec<std::fs::DirEntry> {
    let mut entries: Vec<_> = fs::read_dir(path)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    entries
}

pub(super) fn scan(block: &Path) -> Vec<RawDisk> {
    entries(block)
        .iter()
        .filter_map(|entry| read_disk(block, &entry.file_name().to_string_lossy()))
        .collect()
}

fn read_disk(block: &Path, name: &str) -> Option<RawDisk> {
    let path = block.join(name);
    let hardware = path.join("device");
    if !hardware.exists() {
        return None;
    }
    let size = number::<u64>(&path.join("size"))?.saturating_mul(SECTOR);
    if size == 0 {
        return None; // A card reader or optical drive without a medium.
    }
    let real_path = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    Some(RawDisk {
        name: name.to_owned(),
        device: device_number(&path.join("dev"))?,
        model: model(&hardware),
        size,
        transport: transport(name, &real_path.to_string_lossy()),
        rotational: number::<u8>(&path.join("queue/rotational")) == Some(1),
        removable: number::<u8>(&path.join("removable")) == Some(1),
        temperature: temperature(&hardware),
        link: pcie_link(&hardware.join("device")),
        holders: holders(block, &path),
        partitions: entries(&path)
            .iter()
            .filter_map(|entry| read_partition(block, &entry.path()))
            .collect(),
    })
}

fn read_partition(block: &Path, path: &Path) -> Option<RawPartition> {
    Some(RawPartition {
        number: number(&path.join("partition"))?,
        name: path.file_name()?.to_string_lossy().into_owned(),
        device: device_number(&path.join("dev"))?,
        size: number::<u64>(&path.join("size"))?.saturating_mul(SECTOR),
        holders: holders(block, path),
    })
}

/// One level is enough for the common stacks: the mount sits on the holder.
fn holders(block: &Path, path: &Path) -> Vec<DeviceNumber> {
    entries(&path.join("holders"))
        .iter()
        .filter_map(|entry| device_number(&block.join(entry.file_name()).join("dev")))
        .collect()
}

fn model(hardware: &Path) -> String {
    let vendor = text(&hardware.join("vendor")).filter(|vendor| vendor != "ATA");
    let model = text(&hardware.join("model"));
    match (vendor, model) {
        (Some(vendor), Some(model)) if !model.starts_with(&vendor) => format!("{vendor} {model}"),
        (_, Some(model)) => model,
        (Some(vendor), None) => vendor,
        (None, None) => "Unbekanntes Modell".into(),
    }
}

/// The bus a disk hangs on. A USB enclosure shows as `sdX`, so the device's
/// path through sysfs decides, not its name.
fn transport(name: &str, real_path: &str) -> &'static str {
    if name.starts_with("nvme") {
        "NVMe"
    } else if name.starts_with("mmcblk") {
        "SD/MMC"
    } else if real_path.contains("/usb") {
        "USB"
    } else if real_path.contains("/ata") {
        "SATA"
    } else if real_path.contains("/virtio") {
        "Virtio"
    } else if name.starts_with("sd") {
        "SCSI"
    } else {
        "Unbekannt"
    }
}

/// A flash controller has its sensor directly below it (`hwmonN`), a SATA
/// disk with the `drivetemp` driver below `hwmon/`.
fn temperature(hardware: &Path) -> Option<f32> {
    let direct = entries(hardware);
    let nested = entries(&hardware.join("hwmon"));
    direct
        .iter()
        .chain(&nested)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("hwmon"))
        .find_map(|entry| number::<i32>(&entry.path().join("temp1_input")))
        .map(|milli| (f64::from(milli) / 1000.0) as f32)
        .filter(|celsius| crate::sensors::plausible_temperature(*celsius))
}

/// A flash controller's bus link, from the PCI function behind it.
fn pcie_link(function: &Path) -> Option<PcieLink> {
    let width = |file: &str| number::<u32>(&function.join(file));
    let generation = |file: &str| pcie_generation(&text(&function.join(file))?);
    let link = PcieLink {
        generation: generation("current_link_speed"),
        width: width("current_link_width"),
        max_generation: generation("max_link_speed"),
        max_width: width("max_link_width"),
    };
    (link.generation.is_some() || link.width.is_some()).then_some(link)
}

/// `16.0 GT/s PCIe` is generation 4.
fn pcie_generation(speed: &str) -> Option<u32> {
    let rate: f32 = speed.split_whitespace().next()?.parse().ok()?;
    [2.5, 5.0, 8.0, 16.0, 32.0, 64.0]
        .iter()
        .position(|known| (rate - known).abs() < 0.1)
        .map(|at| at as u32 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Scratch;

    #[test]
    fn reads_a_disk_its_partitions_and_its_sensor() {
        let scratch = Scratch::new("inventory");
        let disk = "block/nvme1n1";
        scratch.write(&format!("{disk}/dev"), "259:5\n");
        scratch.write(&format!("{disk}/size"), "3907029168\n");
        scratch.write(&format!("{disk}/removable"), "0\n");
        scratch.write(&format!("{disk}/queue/rotational"), "0\n");
        let hardware = format!("{disk}/device");
        scratch.write(
            &format!("{hardware}/model"),
            "Samsung SSD 990 PRO 2TB                 \n",
        );
        scratch.write(&format!("{hardware}/hwmon1/temp1_input"), "52850\n");
        let pci = format!("{hardware}/device");
        scratch.write(&format!("{pci}/current_link_speed"), "16.0 GT/s PCIe\n");
        scratch.write(&format!("{pci}/current_link_width"), "4\n");
        scratch.write(&format!("{pci}/max_link_speed"), "16.0 GT/s PCIe\n");
        scratch.write(&format!("{pci}/max_link_width"), "4\n");
        for (name, number, device, sectors) in
            [("p2", 2, "259:7", "3904000000"), ("p1", 1, "259:6", "2048")]
        {
            let part = format!("{disk}/nvme1n1{name}");
            scratch.write(&format!("{part}/partition"), &format!("{number}\n"));
            scratch.write(&format!("{part}/dev"), &format!("{device}\n"));
            scratch.write(&format!("{part}/size"), &format!("{sectors}\n"));
        }
        scratch.write(&format!("{disk}/queue/not_a_partition"), "x");

        let disks = scan(&scratch.path("block"));
        assert_eq!(disks.len(), 1);
        let disk = &disks[0];
        assert_eq!(disk.name, "nvme1n1");
        assert_eq!(disk.device, (259, 5));
        assert_eq!(disk.model, "Samsung SSD 990 PRO 2TB");
        assert_eq!(disk.size, 3_907_029_168 * 512);
        assert_eq!(disk.transport, "NVMe");
        assert!(!disk.rotational);
        assert_eq!(disk.temperature, Some(52.85));
        assert_eq!(
            disk.link,
            Some(PcieLink {
                generation: Some(4),
                width: Some(4),
                max_generation: Some(4),
                max_width: Some(4),
            })
        );
        let names: Vec<_> = disk.partitions.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["nvme1n1p1", "nvme1n1p2"]);
        assert_eq!(disk.partitions[0].size, 2048 * 512);
        assert_eq!(disk.partitions[0].number, 1);
    }

    #[test]
    fn virtual_and_empty_devices_are_no_disks() {
        let scratch = Scratch::new("virtual");
        scratch.write("block/loop0/dev", "7:0\n");
        scratch.write("block/loop0/size", "100\n");
        scratch.write("block/sr0/dev", "11:0\n");
        scratch.write("block/sr0/size", "0\n");
        scratch.write("block/sr0/device/model", "DVD\n");
        assert!(scan(&scratch.path("block")).is_empty());
    }

    #[test]
    fn a_holder_links_a_partition_to_its_mapped_device() {
        let scratch = Scratch::new("holders");
        scratch.write("block/sda/dev", "8:0\n");
        scratch.write("block/sda/size", "1000\n");
        scratch.write("block/sda/device/model", "Disk\n");
        scratch.write("block/sda/sda1/partition", "1\n");
        scratch.write("block/sda/sda1/dev", "8:1\n");
        scratch.write("block/sda/sda1/size", "900\n");
        scratch.write("block/sda/sda1/holders/dm-0", "");
        scratch.write("block/dm-0/dev", "253:0\n");
        scratch.write("block/dm-0/size", "900\n");
        let disks = scan(&scratch.path("block"));
        assert_eq!(disks.len(), 1, "dm-0 is no physical disk");
        assert_eq!(disks[0].partitions[0].holders, [(253, 0)]);
    }

    #[test]
    fn the_transport_follows_the_path_not_the_name() {
        assert_eq!(transport("nvme0n1", "/sys/devices/pci0/nvme/nvme0"), "NVMe");
        assert_eq!(transport("sda", "/sys/devices/pci0/usb2/2-1/host6"), "USB");
        assert_eq!(transport("sda", "/sys/devices/pci0/ata1/host0"), "SATA");
        assert_eq!(transport("mmcblk0", "/sys/devices/mmc0"), "SD/MMC");
        assert_eq!(transport("vda", "/sys/devices/pci0/virtio1"), "Virtio");
        assert_eq!(transport("sdb", "/sys/devices/pci0/host1"), "SCSI");
        assert_eq!(transport("xyz", "/sys/devices/other"), "Unbekannt");
    }

    #[test]
    fn link_speeds_name_their_generation() {
        assert_eq!(pcie_generation("2.5 GT/s PCIe"), Some(1));
        assert_eq!(pcie_generation("8.0 GT/s PCIe"), Some(3));
        assert_eq!(pcie_generation("32.0 GT/s PCIe"), Some(5));
        assert_eq!(pcie_generation("Unknown"), None);
    }

    #[test]
    fn a_sata_model_keeps_a_real_vendor_and_drops_ata() {
        let scratch = Scratch::new("model");
        scratch.write("a/vendor", "ATA     \n");
        scratch.write("a/model", "WDC WD40EFRX\n");
        assert_eq!(model(&scratch.path("a")), "WDC WD40EFRX");
        scratch.write("b/vendor", "SanDisk\n");
        scratch.write("b/model", "Extreme 55AE\n");
        assert_eq!(model(&scratch.path("b")), "SanDisk Extreme 55AE");
        assert_eq!(model(&scratch.path("c")), "Unbekanntes Modell");
    }
}
