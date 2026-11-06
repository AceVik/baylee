//! AMD graphics through the `amdgpu` driver's sysfs and hwmon files, without
//! spawning a tool or opening a device.
//!
//! On an APU the driver's `PPT` sensor is the whole package, the CPU as well as
//! the graphics. So it is handed back as the package's power and the GPU gets
//! no watts at all. `gpu_metrics` has a field called gfx power, but on a Ryzen 7
//! PRO 7840U it read 15.6 W at 0 % graphics activity: it follows the rail the
//! CPU shares, and is not the GPU's own power.

use crate::{
    storage::pcie_link,
    types::{Gpu, PcieLink},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// libdrm's table of marketing names, where the system has one.
pub(super) const NAME_TABLE: &str = "/usr/share/libdrm/amdgpu.ids";

/// What does not change while a driver is loaded, read once per device.
#[derive(Clone, Debug, Default, PartialEq)]
struct Fixed {
    name: String,
    integrated: bool,
    memory_total: Option<u64>,
    gtt_total: Option<u64>,
    clock_graphics_max: Option<u32>,
    clock_memory_max: Option<u32>,
    temp_slowdown: Option<u32>,
    temp_shutdown: Option<u32>,
    pcie: PcieLink,
}

#[derive(Debug)]
pub(super) struct Card {
    device: PathBuf,
    hwmon: Option<PathBuf>,
    fixed: Fixed,
}

fn text(path: &Path) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn number<T: std::str::FromStr>(path: &Path) -> Option<T> {
    text(path)?.parse().ok()
}

/// The driver a device is bound to, from the name of its `driver` link.
pub(crate) fn driver_of(device: &Path) -> Option<String> {
    let link = fs::read_link(device.join("driver")).ok()?;
    Some(link.file_name()?.to_string_lossy().into_owned())
}

/// Every `amdgpu` card under `/sys/class/drm`. The connectors beside them
/// (`card1-eDP-1`) are not cards.
pub(super) fn find(drm: &Path, names: &Path, cpu_brand: &str) -> Vec<Card> {
    let mut paths: Vec<_> = fs::read_dir(drm)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.strip_prefix("card")
                .is_some_and(|number| number.parse::<u32>().is_ok())
        })
        .map(|entry| entry.path().join("device"))
        .filter(|device| driver_of(device).as_deref() == Some("amdgpu"))
        .collect();
    paths.sort();
    let table = fs::read_to_string(names).unwrap_or_default();
    paths
        .into_iter()
        .map(|device| Card::new(device, &table, cpu_brand))
        .collect()
}

impl Card {
    fn new(device: PathBuf, table: &str, cpu_brand: &str) -> Self {
        let hwmon = fs::read_dir(device.join("hwmon"))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .min();
        let integrated = is_integrated(&device);
        let fixed = Fixed {
            name: name_of(&device, table, integrated, cpu_brand),
            integrated,
            memory_total: number(&device.join("mem_info_vram_total")),
            gtt_total: number(&device.join("mem_info_gtt_total")),
            clock_graphics_max: levels(&device.join("pp_dpm_sclk")).and_then(|l| l.max),
            clock_memory_max: levels(&device.join("pp_dpm_mclk")).and_then(|l| l.max),
            temp_slowdown: hwmon.as_ref().and_then(|h| degrees(&h.join("temp1_crit"))),
            temp_shutdown: hwmon
                .as_ref()
                .and_then(|h| degrees(&h.join("temp1_emergency"))),
            // Inside the package there is no slot and no link worth a line.
            pcie: if integrated {
                PcieLink::default()
            } else {
                pcie_link(&device).unwrap_or_default()
            },
        };
        Self {
            device,
            hwmon,
            fixed,
        }
    }

    /// The card as it is now, and the package's power if it is an APU.
    pub fn read(&self, index: u32) -> (Gpu, Option<f64>) {
        let fixed = &self.fixed;
        let mut gpu = Gpu {
            index,
            name: fixed.name.clone(),
            integrated: fixed.integrated,
            memory_total: fixed.memory_total,
            gtt_total: fixed.gtt_total,
            clock_graphics_max: fixed.clock_graphics_max,
            clock_memory_max: fixed.clock_memory_max,
            temp_slowdown: fixed.temp_slowdown,
            temp_shutdown: fixed.temp_shutdown,
            pcie: fixed.pcie.clone(),
            ..Gpu::default()
        };
        // Reading a sensor can wake a sleeping card, which is the one thing a
        // monitor of an idle machine must not do.
        if text(&self.device.join("power/runtime_status")).as_deref() == Some("suspended") {
            gpu.asleep = true;
            return (gpu, None);
        }
        let device = &self.device;
        gpu.utilization = number(&device.join("gpu_busy_percent"));
        gpu.memory_utilization = number(&device.join("mem_busy_percent"));
        gpu.video = number(&device.join("vcn_busy_percent"));
        gpu.memory_used = number(&device.join("mem_info_vram_used"));
        gpu.gtt_used = number(&device.join("mem_info_gtt_used"));
        let hwmon = self.hwmon.as_deref();
        gpu.temperature = hwmon.and_then(|h| degrees(&h.join("temp1_input")));
        gpu.clock_graphics = hwmon
            .and_then(|h| megahertz(&h.join("freq1_input")))
            .or_else(|| levels(&device.join("pp_dpm_sclk")).and_then(|l| l.current));
        gpu.clock_memory = hwmon
            .and_then(|h| megahertz(&h.join("freq2_input")))
            .or_else(|| levels(&device.join("pp_dpm_mclk")).and_then(|l| l.current));
        let watts = hwmon.and_then(power);
        if fixed.integrated {
            return (gpu, watts);
        }
        gpu.watts = watts;
        gpu.power_limit = hwmon.and_then(|h| microwatts(&h.join("power1_cap")));
        gpu.fans = hwmon.and_then(fan_percent).into_iter().collect();
        (gpu, None)
    }
}

/// The driver's own averaged power. The instant reading swung between 1.8 and
/// 28 W at rest around the same 4.2 W mean, which a 1 Hz graph would alias.
fn power(hwmon: &Path) -> Option<f64> {
    microwatts(&hwmon.join("power1_average")).or_else(|| microwatts(&hwmon.join("power1_input")))
}

fn microwatts(path: &Path) -> Option<f64> {
    Some(number::<f64>(path)? / 1_000_000.0)
}

fn megahertz(path: &Path) -> Option<u32> {
    u32::try_from(number::<u64>(path)? / 1_000_000).ok()
}

fn degrees(path: &Path) -> Option<u32> {
    let celsius = number::<f64>(path)? / 1000.0;
    crate::sensors::plausible_temperature(celsius as f32).then(|| celsius.round() as u32)
}

fn fan_percent(hwmon: &Path) -> Option<u32> {
    let speed: f64 = number(&hwmon.join("pwm1"))?;
    let most: f64 = number(&hwmon.join("pwm1_max")).unwrap_or(255.0);
    (most > 0.0).then(|| (100.0 * speed / most).round() as u32)
}

/// The header of `gpu_metrics` names its format: revisions 2 and 3 are the
/// layouts of APUs, 1 is that of discrete cards. The sensor's label cannot
/// tell them apart, since a discrete card's board power is called `PPT` too.
fn is_integrated(device: &Path) -> bool {
    fs::read(device.join("gpu_metrics")).is_ok_and(|metrics| matches!(metrics.get(2), Some(2 | 3)))
}

/// The name a person knows the chip by, from what the machine offers.
fn name_of(device: &Path, table: &str, integrated: bool, cpu_brand: &str) -> String {
    let identifier = |file: &str| {
        let value = text(&device.join(file))?;
        Some(value.trim_start_matches("0x").to_uppercase())
    };
    let (chip, revision) = (identifier("device"), identifier("revision"));
    if let (Some(chip), Some(revision)) = (&chip, &revision)
        && let Some(name) = marketing_name(table, chip, revision)
    {
        return name;
    }
    text(&device.join("product_name"))
        .or_else(|| integrated.then(|| graphics_of_cpu(cpu_brand)).flatten())
        .unwrap_or_else(|| format!("AMD GPU {}", chip.as_deref().unwrap_or("?")))
}

/// One line of the table: chip, revision and name, parted by commas.
fn marketing_name(table: &str, chip: &str, revision: &str) -> Option<String> {
    table.lines().find_map(|line| {
        let mut fields = line.splitn(3, ',').map(str::trim);
        let matches = fields.next()?.eq_ignore_ascii_case(chip)
            && fields.next()?.eq_ignore_ascii_case(revision);
        matches.then(|| fields.next().map(str::to_owned)).flatten()
    })
}

/// `AMD Ryzen 7 PRO 7840U w/ Radeon 780M Graphics` names its own graphics.
fn graphics_of_cpu(brand: &str) -> Option<String> {
    let (_, graphics) = brand.split_once(" w/ ")?;
    Some(graphics.trim().to_owned())
}

/// A `pp_dpm_*` file: the levels the clock can take.
#[derive(Debug, Default, PartialEq, Eq)]
struct Levels {
    current: Option<u32>,
    max: Option<u32>,
}

fn levels(path: &Path) -> Option<Levels> {
    Some(parse_levels(&text(path)?))
}

/// `1: 800Mhz *`: the star marks the level in use, and a repeated clock can
/// carry several stars, which all name the same frequency.
fn parse_levels(text: &str) -> Levels {
    let mut levels = Levels::default();
    for line in text.lines() {
        let Some((_, rest)) = line.split_once(':') else {
            continue;
        };
        let rest = rest.trim();
        let Some(mhz) = rest
            .split_once(|c: char| !c.is_ascii_digit())
            .and_then(|(digits, _)| digits.parse().ok())
        else {
            continue;
        };
        levels.max = levels.max.max(Some(mhz));
        if rest.ends_with('*') && levels.current.is_none() {
            levels.current = Some(mhz);
        }
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Scratch;
    use std::os::unix::fs::symlink;

    const TABLE: &str = "# List of AMDGPU IDs\n\n1.0.0\n15BF,\tDD,\tAMD Radeon 780M Graphics\n73BF,\tC0,\tAMD Radeon RX 6800\n";

    /// `gpu_metrics` starts with a size and the format's revisions.
    fn metrics(format: u8) -> Vec<u8> {
        vec![120, 0, format, 1, 16, 14]
    }

    /// A card the way a Ryzen 7 PRO 7840U shows it, taken from a real machine.
    fn apu(scratch: &Scratch) {
        let device = "pci/0000:64:00.0";
        for (file, value) in [
            ("device", "0x15bf"),
            ("revision", "0xdd"),
            ("gpu_busy_percent", "12"),
            ("vcn_busy_percent", "3"),
            ("mem_info_vram_total", "4294967296"),
            ("mem_info_vram_used", "81514496"),
            ("mem_info_gtt_total", "14521835520"),
            ("mem_info_gtt_used", "14188544"),
            ("pp_dpm_sclk", "0: 800Mhz *\n1: 1100Mhz \n2: 2700Mhz \n"),
            ("pp_dpm_mclk", "0: 400Mhz \n1: 800Mhz *\n2: 800Mhz *\n"),
            ("power/runtime_status", "active"),
            ("hwmon/hwmon8/name", "amdgpu"),
            ("hwmon/hwmon8/temp1_input", "37000"),
            ("hwmon/hwmon8/freq1_input", "800000000"),
            ("hwmon/hwmon8/power1_average", "4170000"),
            ("hwmon/hwmon8/power1_input", "28137000"),
        ] {
            scratch.write(&format!("{device}/{file}"), &format!("{value}\n"));
        }
        fs::write(scratch.path(device).join("gpu_metrics"), metrics(2)).unwrap();
        bind(scratch, device, "drm/card1", "amdgpu");
    }

    fn bind(scratch: &Scratch, device: &str, card: &str, driver: &str) {
        fs::create_dir_all(scratch.path(card)).unwrap();
        symlink(scratch.path(device), scratch.path(card).join("device")).unwrap();
        symlink(
            format!("../../bus/pci/drivers/{driver}"),
            scratch.path(device).join("driver"),
        )
        .unwrap();
    }

    fn discrete(scratch: &Scratch) {
        let device = "pci/0000:03:00.0";
        for (file, value) in [
            ("device", "0x73bf"),
            ("revision", "0xc0"),
            ("gpu_busy_percent", "98"),
            ("mem_busy_percent", "40"),
            ("mem_info_vram_total", "17163091968"),
            ("mem_info_vram_used", "8589934592"),
            ("pp_dpm_sclk", "0: 500Mhz \n1: 2575Mhz *\n"),
            ("current_link_speed", "16.0 GT/s PCIe"),
            ("current_link_width", "16"),
            ("max_link_speed", "16.0 GT/s PCIe"),
            ("max_link_width", "16"),
            ("hwmon/hwmon3/name", "amdgpu"),
            ("hwmon/hwmon3/temp1_input", "71000"),
            ("hwmon/hwmon3/temp1_crit", "100000"),
            ("hwmon/hwmon3/power1_average", "245000000"),
            ("hwmon/hwmon3/power1_cap", "272000000"),
            ("hwmon/hwmon3/pwm1", "128"),
            ("hwmon/hwmon3/pwm1_max", "255"),
        ] {
            scratch.write(&format!("{device}/{file}"), &format!("{value}\n"));
        }
        fs::write(scratch.path(device).join("gpu_metrics"), metrics(1)).unwrap();
        bind(scratch, device, "drm/card0", "amdgpu");
    }

    fn cards(scratch: &Scratch) -> Vec<Card> {
        scratch.write("ids", TABLE);
        find(&scratch.path("drm"), &scratch.path("ids"), "AMD Ryzen 7")
    }

    #[test]
    fn an_apu_shows_its_memory_pools_and_its_power_goes_to_the_package() {
        let scratch = Scratch::new("amd-apu");
        apu(&scratch);
        // A connector of the card is no card, and neither is another driver's.
        fs::create_dir_all(scratch.path("drm/card1-eDP-1")).unwrap();
        let found = cards(&scratch);
        assert_eq!(found.len(), 1);
        let (gpu, package) = found[0].read(2);
        assert_eq!(gpu.index, 2);
        assert_eq!(gpu.name, "AMD Radeon 780M Graphics");
        assert!(gpu.integrated);
        assert_eq!(gpu.utilization, Some(12));
        assert_eq!(gpu.memory_utilization, None, "an APU has no such file");
        assert_eq!(gpu.video, Some(3));
        assert_eq!(gpu.memory_used, Some(81_514_496));
        assert_eq!(gpu.memory_total, Some(4_294_967_296));
        assert_eq!(gpu.gtt_used, Some(14_188_544));
        assert_eq!(gpu.gtt_total, Some(14_521_835_520));
        assert_eq!(gpu.temperature, Some(37));
        assert_eq!(gpu.clock_graphics, Some(800));
        assert_eq!(gpu.clock_graphics_max, Some(2700));
        assert_eq!(gpu.clock_memory, Some(800), "from the starred level");
        assert_eq!(gpu.watts, None, "PPT is the package, not the GPU");
        assert_eq!(package, Some(4.17), "the averaged sensor, not the instant");
        assert_eq!(gpu.pcie, PcieLink::default(), "no link inside a package");
        assert!(!gpu.asleep);
    }

    #[test]
    fn a_discrete_card_keeps_its_own_power_and_link() {
        let scratch = Scratch::new("amd-discrete");
        discrete(&scratch);
        let found = cards(&scratch);
        let (gpu, package) = found[0].read(0);
        assert_eq!(gpu.name, "AMD Radeon RX 6800");
        assert!(!gpu.integrated);
        assert_eq!(gpu.watts, Some(245.0));
        assert_eq!(gpu.power_limit, Some(272.0));
        assert_eq!(package, None, "a card's board power is not the CPU's");
        assert_eq!(gpu.memory_utilization, Some(40));
        assert_eq!(gpu.gtt_used, None);
        assert_eq!(gpu.fans, [50]);
        assert_eq!(gpu.temp_slowdown, Some(100));
        assert_eq!(gpu.pcie.generation, Some(4));
        assert_eq!(gpu.pcie.width, Some(16));
    }

    #[test]
    fn a_sleeping_card_is_not_woken_to_ask_it() {
        let scratch = Scratch::new("amd-asleep");
        discrete(&scratch);
        scratch.write("pci/0000:03:00.0/power/runtime_status", "suspended\n");
        let (gpu, package) = cards(&scratch)[0].read(0);
        assert!(gpu.asleep);
        assert_eq!(gpu.name, "AMD Radeon RX 6800", "what it is stays known");
        assert_eq!(gpu.utilization, None, "unknown, not a guessed zero");
        assert_eq!(gpu.temperature, None);
        assert_eq!(gpu.watts, None);
        assert_eq!(package, None);
    }

    #[test]
    fn cards_of_other_drivers_are_left_alone() {
        let scratch = Scratch::new("amd-other");
        scratch.write("pci/0000:01:00.0/gpu_busy_percent", "1\n");
        bind(&scratch, "pci/0000:01:00.0", "drm/card0", "nvidia");
        assert!(cards(&scratch).is_empty());
        let empty = Scratch::new("amd-none");
        assert!(find(&empty.path("drm"), &empty.path("ids"), "").is_empty());
    }

    #[test]
    fn a_chip_the_table_does_not_know_still_gets_a_name() {
        let scratch = Scratch::new("amd-name");
        let device = scratch.path("device");
        scratch.write("device/device", "0x1900\n");
        scratch.write("device/revision", "0x01\n");
        let brand = "AMD Ryzen 7 8840U w/ Radeon 780M Graphics";
        assert_eq!(
            name_of(&device, "", true, brand),
            "Radeon 780M Graphics",
            "an APU's CPU names it"
        );
        assert_eq!(name_of(&device, "", false, brand), "AMD GPU 1900");
        scratch.write("device/product_name", "Radeon Pro W7900\n");
        assert_eq!(name_of(&device, "", false, brand), "Radeon Pro W7900");
    }

    #[test]
    fn clock_levels_give_the_one_in_use_and_the_top() {
        let levels = parse_levels("0: 400Mhz \n1: 800Mhz *\n2: 800Mhz *\n3: 800Mhz *\n");
        assert_eq!(levels.current, Some(800));
        assert_eq!(levels.max, Some(800));
        let levels = parse_levels("0: 800Mhz *\n1: 1100Mhz \n2: 2700Mhz \n");
        assert_eq!(levels.current, Some(800));
        assert_eq!(levels.max, Some(2700));
        assert_eq!(parse_levels("nonsense\n"), Levels::default());
    }
}
