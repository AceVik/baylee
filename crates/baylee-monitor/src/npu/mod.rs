//! Neural processing units under `/sys/class/accel`: what sysfs says about any
//! of them, and for `amdxdna` what its driver answers when asked.
//!
//! No load is computed here. Where a driver gives no busy figure the field
//! stays empty and the page says so: command counts are not a utilization, and
//! a guessed percentage would be worse than none.

mod xdna;

use crate::{background::now_ms, gpu::driver_of};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

pub(crate) const ACCEL_CLASS: &str = "/sys/class/accel";

/// The newest result of the NPU thread.
#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct NpuView {
    pub checked_ms: u64,
    pub npus: Vec<Npu>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Npu {
    pub index: u32,
    pub name: String,
    pub driver: Option<String>,
    pub firmware: Option<String>,
    /// The PCI power state, `D0` while powered.
    pub pci_state: Option<String>,
    /// Runtime power management: `active` or `suspended`.
    pub runtime: Option<String>,
    /// Suspended, so it was left alone: a question would wake it.
    pub asleep: bool,
    /// The driver answered, so more than sysfs is known.
    pub queried: bool,
    pub columns: Option<u32>,
    pub rows: Option<u32>,
    /// Columns held by contexts: an allocation, not a measure of work.
    pub columns_used: Option<u32>,
    /// The clocks of the level the driver chose, which it does not measure.
    pub clock_mhz: Option<u32>,
    pub clock_control_mhz: Option<u32>,
    pub clock_max_mhz: Option<u32>,
    pub tops_max: Option<u32>,
    pub tasks_max: Option<u32>,
    pub tasks: Option<u32>,
    /// `default`, `low`, `medium`, `high` or `turbo`.
    pub power_mode: Option<&'static str>,
    /// Commands finished per second, over all contexts.
    pub commands_per_s: Option<f64>,
    /// Percent; `None` where the driver has no such sensor.
    pub busy: Option<f32>,
    /// `None` where the driver has no such sensor.
    pub watts: Option<f64>,
    pub contexts: Vec<NpuContext>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub(crate) struct NpuContext {
    pub pid: u32,
    pub columns: u32,
    pub submitted: u64,
    pub completed: u64,
    pub errors: u64,
    pub active: bool,
}

fn text(path: &Path) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/// `accel0` and so on, by number, which is also the device node's.
fn devices(class: &Path) -> Vec<(u32, PathBuf)> {
    let mut found: Vec<_> = fs::read_dir(class)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let index = name.to_string_lossy().strip_prefix("accel")?.parse().ok()?;
            Some((index, entry.path().join("device")))
        })
        .collect();
    found.sort();
    found
}

/// Whether this machine has one. Without it there is no thread at all.
pub(crate) fn present(class: &Path) -> bool {
    !devices(class).is_empty()
}

/// What sysfs says, which costs the device nothing.
fn read_sysfs(device: &Path, index: u32) -> Npu {
    let runtime = text(&device.join("power/runtime_status"));
    Npu {
        index,
        name: text(&device.join("vbnv")).unwrap_or_else(|| format!("NPU {index}")),
        driver: driver_of(device),
        firmware: text(&device.join("fw_version")),
        pci_state: text(&device.join("power_state")),
        asleep: runtime.as_deref() == Some("suspended"),
        runtime,
        ..Npu::default()
    }
}

/// Asks the driver, if it is one this module knows and the device is awake.
/// An `ioctl` numbered for `amdxdna` means something else to another driver.
fn read_device(device: &Path, index: u32, ask: impl FnOnce(&Path) -> Option<xdna::Answers>) -> Npu {
    let mut npu = read_sysfs(device, index);
    if npu.driver.as_deref() == Some("amdxdna") && !npu.asleep {
        let node = PathBuf::from(format!("/dev/accel/accel{index}"));
        if let Some(answers) = ask(&node) {
            answers.apply(&mut npu);
        }
    }
    npu
}

/// Commands finished so far, per device, and when that was counted.
type Counted = BTreeMap<u32, (u64, Instant)>;

pub(crate) struct Npus {
    class: PathBuf,
    counted: Counted,
}

impl Npus {
    pub fn new(class: &Path) -> Self {
        Self {
            class: class.to_owned(),
            counted: Counted::new(),
        }
    }

    pub fn sample(&mut self) -> NpuView {
        let npus = devices(&self.class)
            .into_iter()
            .map(|(index, device)| {
                let mut npu = read_device(&device, index, xdna::ask);
                add_rate(&mut self.counted, &mut npu, Instant::now());
                npu
            })
            .collect();
        NpuView {
            checked_ms: now_ms(),
            npus,
        }
    }
}

/// Fills in the rate. A context that ended takes its count with it, so a total
/// that fell starts over instead of reading as a huge jump.
#[allow(clippy::cast_precision_loss)] // A rate, from a difference of counters.
fn add_rate(counted: &mut Counted, npu: &mut Npu, now: Instant) {
    if !npu.queried {
        counted.remove(&npu.index);
        return;
    }
    let total: u64 = npu.contexts.iter().map(|context| context.completed).sum();
    if let Some((before, at)) = counted.insert(npu.index, (total, now)) {
        let elapsed = now.duration_since(at).as_secs_f64();
        if elapsed > 0.0 {
            npu.commands_per_s = Some(total.saturating_sub(before) as f64 / elapsed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Scratch;
    use std::{os::unix::fs::symlink, time::Duration};

    /// `accel0` as a Ryzen 7 PRO 7840U shows it.
    fn npu1(scratch: &Scratch) {
        let device = "pci/0000:65:00.1";
        for (file, value) in [
            ("vbnv", "RyzenAI-npu1"),
            ("fw_version", "1.5.5.391"),
            ("power_state", "D0"),
            ("power/runtime_status", "active"),
        ] {
            scratch.write(&format!("{device}/{file}"), &format!("{value}\n"));
        }
        fs::create_dir_all(scratch.path("class/accel0")).unwrap();
        symlink(scratch.path(device), scratch.path("class/accel0/device")).unwrap();
        symlink(
            "../../bus/pci/drivers/amdxdna",
            scratch.path(device).join("driver"),
        )
        .unwrap();
    }

    fn device(scratch: &Scratch) -> PathBuf {
        scratch.path("class/accel0/device")
    }

    #[test]
    fn sysfs_names_the_device_and_its_state_without_asking_it() {
        let scratch = Scratch::new("npu-sysfs");
        npu1(&scratch);
        assert!(present(&scratch.path("class")));
        let npu = read_device(&device(&scratch), 0, |_| None);
        assert_eq!(npu.name, "RyzenAI-npu1");
        assert_eq!(npu.driver.as_deref(), Some("amdxdna"));
        assert_eq!(npu.firmware.as_deref(), Some("1.5.5.391"));
        assert_eq!(npu.pci_state.as_deref(), Some("D0"));
        assert_eq!(npu.runtime.as_deref(), Some("active"));
        assert!(!npu.asleep);
        assert!(!npu.queried, "a closed node leaves only sysfs");
        assert_eq!(npu.busy, None);
        assert_eq!(npu.watts, None);
    }

    #[test]
    fn the_answers_of_the_driver_are_added() {
        let scratch = Scratch::new("npu-answers");
        npu1(&scratch);
        let npu = read_device(&device(&scratch), 0, |node| {
            assert_eq!(node, Path::new("/dev/accel/accel0"));
            Some(xdna::Answers {
                columns: Some(5),
                tasks: Some(0),
                ..xdna::Answers::default()
            })
        });
        assert!(npu.queried);
        assert_eq!(npu.columns, Some(5));
        assert_eq!(npu.tasks, Some(0));
        assert_eq!(npu.columns_used, Some(0), "known: no context holds any");
    }

    #[test]
    fn a_sleeping_npu_is_not_woken_to_ask_it() {
        let scratch = Scratch::new("npu-asleep");
        npu1(&scratch);
        scratch.write("pci/0000:65:00.1/power/runtime_status", "suspended\n");
        let npu = read_device(&device(&scratch), 0, |_| panic!("the question wakes it"));
        assert!(npu.asleep);
        assert_eq!(npu.name, "RyzenAI-npu1", "what it is stays known");
    }

    #[test]
    fn another_drivers_device_never_gets_this_drivers_ioctls() {
        let scratch = Scratch::new("npu-other");
        scratch.write("pci/0000:00:0b.0/power/runtime_status", "active\n");
        fs::create_dir_all(scratch.path("class/accel1")).unwrap();
        symlink(
            scratch.path("pci/0000:00:0b.0"),
            scratch.path("class/accel1/device"),
        )
        .unwrap();
        symlink(
            "../../bus/pci/drivers/intel_vpu",
            scratch.path("pci/0000:00:0b.0/driver"),
        )
        .unwrap();
        let path = scratch.path("class/accel1/device");
        let npu = read_device(&path, 1, |_| panic!("wrong driver"));
        assert_eq!(npu.name, "NPU 1", "no name offered, so its number");
        assert_eq!(npu.driver.as_deref(), Some("intel_vpu"));
    }

    #[test]
    fn a_machine_without_one_has_none() {
        let scratch = Scratch::new("npu-none");
        assert!(!present(&scratch.path("class")));
        assert!(Npus::new(&scratch.path("class")).sample().npus.is_empty());
    }

    #[test]
    fn devices_come_by_number_and_other_entries_are_skipped() {
        let scratch = Scratch::new("npu-order");
        for name in ["accel10", "accel2", "accelx", "version"] {
            fs::create_dir_all(scratch.path(&format!("class/{name}"))).unwrap();
        }
        let numbers: Vec<_> = devices(&scratch.path("class"))
            .into_iter()
            .map(|(index, _)| index)
            .collect();
        assert_eq!(numbers, [2, 10]);
    }

    #[test]
    fn the_rate_needs_two_counts_and_survives_a_context_ending() {
        let mut counted = Counted::new();
        let start = Instant::now();
        let mut npu = |completed: Option<u64>, seconds: u64| {
            let mut npu = Npu {
                queried: completed.is_some(),
                contexts: completed
                    .map(|completed| NpuContext {
                        completed,
                        ..NpuContext::default()
                    })
                    .into_iter()
                    .collect(),
                ..Npu::default()
            };
            add_rate(&mut counted, &mut npu, start + Duration::from_secs(seconds));
            npu.commands_per_s
        };
        assert_eq!(npu(Some(100), 0), None, "the first count has no before");
        assert_eq!(npu(Some(150), 5), Some(10.0));
        assert_eq!(npu(Some(20), 10), Some(0.0), "not a negative rate");
        assert_eq!(npu(None, 15), None, "the driver stopped answering");
        assert_eq!(npu(Some(40), 20), None, "and that started over");
    }
}
