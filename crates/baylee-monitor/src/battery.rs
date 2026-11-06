//! Batteries and mains power from `/sys/class/power_supply`, on a thread of
//! their own: `AC/online` alone took 0.35 ms through the embedded controller.
//! Serial numbers and models are left unread; a level needs no identity.

use crate::background::now_ms;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// A discharge below this is noise, and would forecast days of runtime.
const MIN_WATTS: f64 = 0.5;

/// The newest result of the battery thread.
#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct PowerView {
    /// When this view was assembled; zero before the first refresh.
    pub checked_ms: u64,
    pub batteries: Vec<Battery>,
    /// Whether mains is connected; `None` where no supply says.
    pub on_mains: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Battery {
    pub name: String,
    pub percent: Option<f32>,
    /// `charging`, `discharging`, `full`, `not_charging` or `unknown`.
    pub state: &'static str,
    pub energy_wh: Option<f64>,
    pub full_wh: Option<f64>,
    pub design_wh: Option<f64>,
    /// Flow in or out; the driver reports zero while it is neither.
    pub watts: Option<f64>,
    pub volts: Option<f64>,
    pub cycles: Option<u32>,
    /// Where the firmware stops charging, to spare the cells.
    pub charge_limit: Option<u32>,
    pub empty_in_secs: Option<u64>,
    pub charged_in_secs: Option<u64>,
}

fn text(path: &Path) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn number<T: std::str::FromStr>(path: &Path) -> Option<T> {
    text(path)?.parse().ok()
}

fn supplies(root: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        // A wireless mouse's battery belongs to the mouse, not to the machine.
        .filter(|path| text(&path.join("scope")).as_deref() != Some("Device"))
        .collect();
    paths.sort();
    paths
}

fn is_battery(supply: &Path) -> bool {
    text(&supply.join("type")).as_deref() == Some("Battery")
        && number::<u32>(&supply.join("present")).is_none_or(|present| present > 0)
}

/// Whether this machine runs on a battery at all. A desktop has no reason to
/// keep a thread for this.
pub(crate) fn present(root: &Path) -> bool {
    supplies(root).iter().any(|supply| is_battery(supply))
}

pub(crate) fn read(root: &Path) -> PowerView {
    let mut batteries = Vec::new();
    let mut on_mains = None;
    for supply in supplies(root) {
        if is_battery(&supply) {
            batteries.push(read_battery(&supply));
        } else if matches!(text(&supply.join("type")).as_deref(), Some("Mains" | "USB"))
            && let Some(online) = number::<u32>(&supply.join("online"))
        {
            // Several ports may report; one that is live is enough.
            on_mains = Some(on_mains.unwrap_or(false) || online > 0);
        }
    }
    PowerView {
        checked_ms: now_ms(),
        batteries,
        on_mains,
    }
}

fn state_of(status: Option<&str>) -> &'static str {
    match status {
        Some("Charging") => "charging",
        Some("Discharging") => "discharging",
        Some("Full") => "full",
        Some("Not charging") => "not_charging",
        _ => "unknown",
    }
}

fn read_battery(supply: &Path) -> Battery {
    let micro = |file: &str| Some(number::<f64>(&supply.join(file))? / 1_000_000.0);
    let volts = micro("voltage_now");
    // Some drivers count charge instead of energy: `charge_*` in µAh, and
    // `current_now` in µA, which the voltage turns into the same units.
    let energy = |name: &str| {
        micro(&format!("energy_{name}"))
            .or_else(|| Some(micro(&format!("charge_{name}"))? * volts?))
    };
    let energy_wh = energy("now");
    let full_wh = energy("full");
    let watts = micro("power_now")
        .or_else(|| Some(micro("current_now")? * volts?))
        .map(f64::abs);
    let state = state_of(text(&supply.join("status")).as_deref());
    let percent = number::<f32>(&supply.join("capacity"))
        .or_else(|| Some((100.0 * energy_wh? / full_wh?) as f32).filter(|value| value.is_finite()));
    let charge_limit = number(&supply.join("charge_control_end_threshold"));
    let (empty_in_secs, charged_in_secs) = forecast(state, energy_wh, full_wh, watts, charge_limit);
    Battery {
        name: supply
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        percent,
        state,
        energy_wh,
        full_wh,
        design_wh: energy("full_design"),
        watts,
        volts,
        cycles: number(&supply.join("cycle_count")),
        charge_limit,
        empty_in_secs,
        charged_in_secs,
    }
}

/// Seconds until empty, or until charging stops. Charging ends at the
/// firmware's limit, not at the full capacity.
fn forecast(
    state: &str,
    energy_wh: Option<f64>,
    full_wh: Option<f64>,
    watts: Option<f64>,
    limit: Option<u32>,
) -> (Option<u64>, Option<u64>) {
    let (Some(energy), Some(watts)) = (energy_wh, watts.filter(|w| *w >= MIN_WATTS)) else {
        return (None, None);
    };
    let seconds = |watt_hours: f64| (watt_hours / watts * 3600.0).max(0.0) as u64;
    match state {
        "discharging" => (Some(seconds(energy)), None),
        "charging" => {
            let target =
                full_wh.map(|full| full * f64::from(limit.unwrap_or(100).min(100)) / 100.0);
            (None, target.map(|target| seconds(target - energy)))
        }
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Scratch;

    /// `BAT0` and `AC` the way a real notebook showed them, on mains and held
    /// at its charge limit.
    fn thinkpad(scratch: &Scratch) {
        for (file, value) in [
            ("BAT0/type", "Battery"),
            ("BAT0/present", "1"),
            ("BAT0/status", "Not charging"),
            ("BAT0/capacity", "76"),
            ("BAT0/energy_now", "40310000"),
            ("BAT0/energy_full", "53030000"),
            ("BAT0/energy_full_design", "52500000"),
            ("BAT0/power_now", "0"),
            ("BAT0/voltage_now", "15359000"),
            ("BAT0/cycle_count", "19"),
            ("BAT0/charge_control_end_threshold", "80"),
            ("BAT0/serial_number", "18236"),
            ("BAT0/model_name", "5B10W51867"),
            ("AC/type", "Mains"),
            ("AC/online", "1"),
        ] {
            scratch.write(&format!("supply/{file}"), &format!("{value}\n"));
        }
    }

    #[test]
    fn a_notebook_on_mains_reports_level_state_and_health() {
        let scratch = Scratch::new("battery");
        thinkpad(&scratch);
        let root = scratch.path("supply");
        assert!(present(&root));
        let view = read(&root);
        assert_eq!(view.on_mains, Some(true));
        assert_eq!(view.batteries.len(), 1);
        let battery = &view.batteries[0];
        assert_eq!(battery.name, "BAT0");
        assert_eq!(battery.percent, Some(76.0));
        assert_eq!(battery.state, "not_charging");
        assert_eq!(battery.energy_wh, Some(40.31));
        assert_eq!(battery.full_wh, Some(53.03));
        assert_eq!(battery.design_wh, Some(52.5));
        assert_eq!(battery.watts, Some(0.0));
        assert_eq!(battery.cycles, Some(19));
        assert_eq!(battery.charge_limit, Some(80));
        assert_eq!(battery.empty_in_secs, None, "it is not draining");
        assert_eq!(battery.charged_in_secs, None, "nor charging");
    }

    #[test]
    fn nothing_identifying_the_battery_reaches_the_wire() {
        let scratch = Scratch::new("battery-wire");
        thinkpad(&scratch);
        let json = serde_json::to_string(&read(&scratch.path("supply"))).unwrap();
        assert!(!json.contains("18236"), "the serial number: {json}");
        assert!(!json.contains("5B10W51867"), "the model: {json}");
    }

    #[test]
    fn draining_forecasts_the_end_and_charging_stops_at_the_limit() {
        let scratch = Scratch::new("battery-flow");
        thinkpad(&scratch);
        scratch.write("supply/AC/online", "0\n");
        scratch.write("supply/BAT0/status", "Discharging\n");
        scratch.write("supply/BAT0/power_now", "10000000\n");
        let view = read(&scratch.path("supply"));
        assert_eq!(view.on_mains, Some(false));
        // 40.31 Wh at 10 W is 4.031 hours.
        assert_eq!(view.batteries[0].empty_in_secs, Some(14_511));

        scratch.write("supply/AC/online", "1\n");
        scratch.write("supply/BAT0/status", "Charging\n");
        let view = read(&scratch.path("supply"));
        assert_eq!(view.batteries[0].empty_in_secs, None);
        // 80 % of 53.03 Wh is 42.424 Wh, 2.114 Wh away at 10 W.
        assert_eq!(view.batteries[0].charged_in_secs, Some(761));
    }

    #[test]
    fn a_trickle_forecasts_nothing() {
        assert_eq!(
            forecast("discharging", Some(40.0), Some(50.0), Some(0.1), None),
            (None, None),
            "400 hours would be a guess"
        );
    }

    #[test]
    fn a_driver_that_counts_charge_is_converted() {
        let scratch = Scratch::new("battery-charge");
        for (file, value) in [
            ("type", "Battery"),
            ("status", "Discharging"),
            ("charge_now", "2000000"),
            ("charge_full", "4000000"),
            ("current_now", "-1000000"),
            ("voltage_now", "12000000"),
        ] {
            scratch.write(&format!("supply/BAT1/{file}"), &format!("{value}\n"));
        }
        let battery = read(&scratch.path("supply")).batteries.remove(0);
        assert_eq!(battery.energy_wh, Some(24.0));
        assert_eq!(battery.full_wh, Some(48.0));
        assert_eq!(battery.watts, Some(12.0), "a negative current still flows");
        assert_eq!(battery.percent, Some(50.0), "no capacity file: computed");
        assert_eq!(battery.empty_in_secs, Some(7200));
    }

    #[test]
    fn a_mouse_and_an_empty_slot_are_not_the_machines_battery() {
        let scratch = Scratch::new("battery-other");
        scratch.write("supply/hid-mouse/type", "Battery\n");
        scratch.write("supply/hid-mouse/scope", "Device\n");
        scratch.write("supply/hid-mouse/capacity", "40\n");
        scratch.write("supply/BAT1/type", "Battery\n");
        scratch.write("supply/BAT1/present", "0\n");
        let root = scratch.path("supply");
        assert!(!present(&root));
        assert!(read(&root).batteries.is_empty());
    }

    #[test]
    fn a_desktop_has_no_battery_and_no_opinion_on_mains() {
        let scratch = Scratch::new("battery-none");
        let root = scratch.path("supply");
        assert!(!present(&root));
        let view = read(&root);
        assert!(view.batteries.is_empty());
        assert_eq!(view.on_mains, None);
    }

    #[test]
    fn one_live_port_is_mains() {
        let scratch = Scratch::new("battery-ports");
        scratch.write("supply/ucsi-source-psy-1/type", "USB\n");
        scratch.write("supply/ucsi-source-psy-1/online", "0\n");
        scratch.write("supply/ucsi-source-psy-2/type", "USB\n");
        scratch.write("supply/ucsi-source-psy-2/online", "1\n");
        assert_eq!(read(&scratch.path("supply")).on_mains, Some(true));
    }
}
