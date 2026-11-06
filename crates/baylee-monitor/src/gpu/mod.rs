//! The machine's graphics chips, from whatever source each one has: NVML for
//! NVIDIA, sysfs for AMD. Detected once at startup, so a machine without one
//! costs nothing and shows nothing.

mod amd;
mod nvidia;

pub(crate) use amd::driver_of;

use crate::{background::now_ms, types::GpuView};
use nvidia::Nvidia;
use std::{fs, path::Path};

const DISPLAY_CONTROLLER: &str = "0x03";
const NVIDIA_VENDOR: &str = "0x10de";

pub(crate) struct Gpus {
    nvidia: Option<Nvidia>,
    amd: Vec<amd::Card>,
    /// A card is in the machine but its driver does not answer. That is a fault
    /// to show, unlike a machine that simply has no such card.
    problem: Option<String>,
}

impl Gpus {
    pub fn new(cpu_brand: &str) -> Self {
        let nvidia = Nvidia::new();
        let problem = (nvidia.is_none() && has_nvidia_card(Path::new("/sys/bus/pci/devices")))
            .then(|| "NVIDIA-Karte vorhanden, aber NVML antwortet nicht".to_owned());
        Self {
            nvidia,
            amd: amd::find(
                Path::new("/sys/class/drm"),
                Path::new(amd::NAME_TABLE),
                cpu_brand,
            ),
            problem,
        }
    }

    /// No source at all: nothing to sample and nothing to show.
    pub fn is_empty(&self) -> bool {
        self.nvidia.is_none() && self.amd.is_empty() && self.problem.is_none()
    }

    pub fn sample(&mut self) -> GpuView {
        let mut view = self.nvidia.as_mut().map_or_else(
            || GpuView {
                status: self.problem.clone().unwrap_or_default(),
                ..GpuView::default()
            },
            Nvidia::sample,
        );
        view.checked_ms = now_ms();
        if self.amd.is_empty() {
            return view;
        }
        let first = view.gpus.len() as u32;
        for (index, card) in (first..).zip(&self.amd) {
            let (gpu, package) = card.read(index);
            view.gpus.push(gpu);
            view.package_watts = view.package_watts.or(package);
        }
        view.driver.get_or_insert_with(|| "amdgpu".to_owned());
        view
    }
}

/// An NVIDIA display controller on the bus, whatever its driver is doing.
fn has_nvidia_card(devices: &Path) -> bool {
    let attribute = |device: &Path, file: &str| fs::read_to_string(device.join(file));
    fs::read_dir(devices)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|entry| {
            let device = entry.path();
            attribute(&device, "vendor").is_ok_and(|vendor| vendor.trim() == NVIDIA_VENDOR)
                && attribute(&device, "class")
                    .is_ok_and(|class| class.trim().starts_with(DISPLAY_CONTROLLER))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Scratch;

    #[test]
    fn only_an_nvidia_display_controller_counts_as_a_card() {
        let scratch = Scratch::new("gpu-pci");
        scratch.write("pci/0000:00:14.0/vendor", "0x1022\n");
        scratch.write("pci/0000:00:14.0/class", "0x030000\n");
        scratch.write("pci/0000:01:00.1/vendor", "0x10de\n");
        scratch.write("pci/0000:01:00.1/class", "0x040300\n");
        assert!(!has_nvidia_card(&scratch.path("pci")), "its audio function");
        scratch.write("pci/0000:01:00.0/vendor", "0x10de\n");
        scratch.write("pci/0000:01:00.0/class", "0x030000\n");
        assert!(has_nvidia_card(&scratch.path("pci")));
    }

    #[test]
    fn a_machine_without_any_source_is_empty_and_quiet() {
        let mut gpus = Gpus {
            nvidia: None,
            amd: Vec::new(),
            problem: None,
        };
        assert!(gpus.is_empty());
        let view = gpus.sample();
        assert!(view.gpus.is_empty());
        assert_eq!(view.status, "", "no card is no error");
    }

    #[test]
    fn a_card_whose_driver_is_gone_stays_a_fault() {
        let mut gpus = Gpus {
            nvidia: None,
            amd: Vec::new(),
            problem: Some("NVIDIA-Karte vorhanden, aber NVML antwortet nicht".into()),
        };
        assert!(!gpus.is_empty());
        assert!(gpus.sample().status.contains("NVML"));
    }
}
