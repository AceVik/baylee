//! Sensors in `/sys/class/hwmon` and CPU frequencies, on
//! their own thread: an embedded controller or a busy driver may take tens of
//! milliseconds per value, and the 1 Hz sampler must not wait for that.

use crate::background::now_ms;
use serde::Serialize;
use std::{fs, path::Path};

/// More would not fit on a phone, and no board has that many useful ones.
const MAX_READINGS: usize = 96;

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct SensorView {
    /// When this view was assembled; zero before the first refresh.
    pub checked_ms: u64,
    pub readings: Vec<Reading>,
    pub cpu_temp: Option<f32>,
    /// Each logical CPU's current frequency, by CPU number.
    pub freq_mhz: Vec<u32>,
    pub freq_min_mhz: Option<u32>,
    pub freq_max_mhz: Option<u32>,
    pub governor: Option<String>,
    pub freq_driver: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Reading {
    /// The driver's name; drives all report `nvme`, so theirs is the controller's.
    pub chip: String,
    pub label: String,
    /// `temp`, `fan`, `voltage`, `current` or `power`.
    pub kind: &'static str,
    pub value: f32,
    pub max: Option<f32>,
    pub crit: Option<f32>,
}

pub(crate) fn plausible_temperature(celsius: f32) -> bool {
    celsius.is_finite() && celsius > -20.0 && celsius < 150.0
}

pub(crate) fn read(threads: usize) -> SensorView {
    let readings = read_hwmon(Path::new("/sys/class/hwmon"));
    let cpu_root = Path::new("/sys/devices/system/cpu");
    let policy = cpu_root.join("cpu0/cpufreq");
    let khz = |file: &str| {
        let text = fs::read_to_string(policy.join(file)).ok()?;
        Some(text.trim().parse::<u32>().ok()? / 1000)
    };
    let word = |file: &str| {
        Some(
            fs::read_to_string(policy.join(file))
                .ok()?
                .trim()
                .to_owned(),
        )
    };
    SensorView {
        checked_ms: now_ms(),
        cpu_temp: cpu_temperature(&readings),
        readings,
        freq_mhz: read_frequencies(cpu_root, threads),
        freq_min_mhz: khz("cpuinfo_min_freq"),
        freq_max_mhz: khz("cpuinfo_max_freq"),
        governor: word("scaling_governor"),
        freq_driver: word("scaling_driver"),
    }
}

fn read_frequencies(cpu_root: &Path, threads: usize) -> Vec<u32> {
    let frequencies: Vec<_> = (0..threads)
        .map_while(|cpu| {
            let path = cpu_root.join(format!("cpu{cpu}/cpufreq/scaling_cur_freq"));
            Some(fs::read_to_string(path).ok()?.trim().parse::<u32>().ok()? / 1000)
        })
        .collect();
    // All or nothing: a partial list would shift no index, but says less.
    if frequencies.len() == threads {
        frequencies
    } else {
        Vec::new()
    }
}

fn read_hwmon(root: &Path) -> Vec<Reading> {
    let mut chips: Vec<_> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    chips.sort();
    let mut readings = Vec::new();
    for chip in chips {
        let Some(name) = fs::read_to_string(chip.join("name")).ok() else {
            continue;
        };
        let real = fs::canonicalize(&chip).unwrap_or_else(|_| chip.clone());
        let chip_name = chip_name(name.trim(), &real.to_string_lossy());
        read_chip(&chip, &chip_name, &mut readings);
    }
    readings.truncate(MAX_READINGS);
    readings
}

/// Three drives all call themselves `nvme`; the controller in the path tells
/// them apart (`…/nvme/nvme0/hwmon0`).
fn chip_name(name: &str, real_path: &str) -> String {
    if name == "nvme"
        && let Some(controller) = real_path
            .rsplit('/')
            .find(|part| part.starts_with("nvme") && part.len() > "nvme".len())
    {
        return controller.to_owned();
    }
    name.to_owned()
}

fn read_chip(chip: &Path, chip_name: &str, readings: &mut Vec<Reading>) {
    let mut files: Vec<_> = fs::read_dir(chip)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter_map(|file| {
            let stem = file.strip_suffix("_input")?.to_owned();
            let (kind, scale) = classify(&stem)?;
            let index: u32 = stem.trim_start_matches(char::is_alphabetic).parse().ok()?;
            Some((kind, index, stem, scale))
        })
        .collect();
    files.sort_by_key(|(kind, index, ..)| (*kind, *index));
    for (kind, _, stem, scale) in files {
        let value = |suffix: &str| {
            let text = fs::read_to_string(chip.join(format!("{stem}_{suffix}"))).ok()?;
            Some(text.trim().parse::<f64>().ok()? / scale)
        };
        let Some(reading) = value("input") else {
            continue; // A sensor may refuse to answer: `-ENODATA`, not a zero.
        };
        let reading = reading as f32;
        if kind == "temp" && !plausible_temperature(reading) {
            continue;
        }
        let limit = |suffix: &str| {
            value(suffix)
                .map(|limit| limit as f32)
                .filter(|limit| *limit > 0.0 && (kind != "temp" || plausible_temperature(*limit)))
        };
        let label = fs::read_to_string(chip.join(format!("{stem}_label")))
            .map_or_else(|_| stem.clone(), |label| label.trim().to_owned());
        readings.push(Reading {
            chip: chip_name.to_owned(),
            label,
            kind,
            value: reading,
            max: limit("max"),
            crit: limit("crit"),
        });
    }
}

/// A hwmon file stem's kind, and what divides its value into base units.
fn classify(stem: &str) -> Option<(&'static str, f64)> {
    let family = stem.trim_end_matches(|c: char| c.is_ascii_digit());
    match family {
        "temp" => Some(("temp", 1000.0)),
        "fan" => Some(("fan", 1.0)),
        "in" => Some(("voltage", 1000.0)),
        "curr" => Some(("current", 1000.0)),
        "power" => Some(("power", 1_000_000.0)),
        _ => None,
    }
}

/// The die temperature, by the names the common drivers use.
fn cpu_temperature(readings: &[Reading]) -> Option<f32> {
    const KNOWN: [(&str, &str); 5] = [
        ("k10temp", "Tctl"),
        ("k10temp", "Tdie"),
        ("zenpower", "Tdie"),
        ("coretemp", "Package id 0"),
        ("cpu_thermal", "temp1"),
    ];
    KNOWN.iter().find_map(|(chip, label)| {
        readings
            .iter()
            .find(|r| r.kind == "temp" && r.chip == *chip && r.label == *label)
            .map(|r| r.value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Scratch;

    #[test]
    fn reads_every_kind_with_labels_and_limits() {
        let scratch = Scratch::new("hwmon");
        let chip = "hwmon/hwmon5";
        scratch.write(&format!("{chip}/name"), "asusec\n");
        scratch.write(&format!("{chip}/temp1_input"), "77000\n");
        scratch.write(&format!("{chip}/temp1_label"), "Chipset\n");
        scratch.write(&format!("{chip}/temp1_max"), "95000\n");
        scratch.write(&format!("{chip}/temp1_crit"), "105000\n");
        scratch.write(&format!("{chip}/temp4_input"), "-40000\n");
        scratch.write(&format!("{chip}/temp4_label"), "T_Sensor\n");
        scratch.write(&format!("{chip}/fan2_input"), "3756\n");
        scratch.write(&format!("{chip}/fan2_label"), "Chipset\n");
        scratch.write(&format!("{chip}/in0_input"), "1271\n");
        scratch.write(&format!("{chip}/curr1_input"), "38000\n");
        scratch.write(&format!("{chip}/power1_input"), "45500000\n");
        scratch.write(&format!("{chip}/uevent"), "x\n");
        let readings = read_hwmon(&scratch.path("hwmon"));
        let find = |kind: &str| readings.iter().find(|r| r.kind == kind).unwrap();
        assert_eq!(readings.len(), 5, "the -40 °C placeholder is dropped");
        let temp = find("temp");
        assert_eq!(temp.chip, "asusec");
        assert_eq!(temp.label, "Chipset");
        assert!((temp.value - 77.0).abs() < 0.01);
        assert_eq!(temp.max, Some(95.0));
        assert_eq!(temp.crit, Some(105.0));
        assert!((find("fan").value - 3756.0).abs() < 0.01);
        assert!((find("voltage").value - 1.271).abs() < 0.001);
        assert_eq!(
            find("voltage").label,
            "in0",
            "no label file: the file's name"
        );
        assert!((find("current").value - 38.0).abs() < 0.01);
        assert!((find("power").value - 45.5).abs() < 0.01);
    }

    #[test]
    fn a_sensor_that_refuses_to_answer_is_left_out() {
        let scratch = Scratch::new("hwmon-empty");
        scratch.write("hwmon/hwmon0/name", "broken\n");
        scratch.write("hwmon/hwmon0/temp1_input", "");
        scratch.write("hwmon/hwmon1/uevent", "no name file\n");
        assert!(read_hwmon(&scratch.path("hwmon")).is_empty());
    }

    #[test]
    fn nvme_chips_are_named_by_their_controller() {
        let path = "/sys/devices/pci0000:00/0000:01:00.0/nvme/nvme2/hwmon2";
        assert_eq!(chip_name("nvme", path), "nvme2");
        assert_eq!(
            chip_name("k10temp", "/sys/devices/pci0/hwmon/hwmon4"),
            "k10temp"
        );
    }

    #[test]
    fn the_cpu_temperature_follows_the_driver() {
        let reading = |chip: &str, label: &str, value| Reading {
            chip: chip.into(),
            label: label.into(),
            kind: "temp",
            value,
            max: None,
            crit: None,
        };
        let readings = [
            reading("nvme0", "Composite", 59.0),
            reading("k10temp", "Tccd1", 61.0),
            reading("k10temp", "Tctl", 60.0),
        ];
        assert_eq!(cpu_temperature(&readings), Some(60.0));
        assert_eq!(cpu_temperature(&readings[..2]), None);
        let intel = [reading("coretemp", "Package id 0", 44.0)];
        assert_eq!(cpu_temperature(&intel), Some(44.0));
    }

    #[test]
    fn frequencies_are_all_or_nothing() {
        let scratch = Scratch::new("cpufreq");
        scratch.write("cpu/cpu0/cpufreq/scaling_cur_freq", "4099109\n");
        scratch.write("cpu/cpu1/cpufreq/scaling_cur_freq", "2200000\n");
        assert_eq!(read_frequencies(&scratch.path("cpu"), 2), [4099, 2200]);
        assert!(read_frequencies(&scratch.path("cpu"), 3).is_empty());
    }
}
