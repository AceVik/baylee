//! Per-interface traffic from `/proc/net/dev` and link facts from
//! `/sys/class/net`. Addresses are not read at all: the page has no login, and
//! a global IPv6 or MAC address says more about a network than a LAN page
//! should.

use crate::{
    procfs::{
        netdev::{self, Counters},
        per_second,
    },
    types::Network,
};
use std::{collections::BTreeMap, fs, path::Path};

/// A container host can have hundreds of veth pairs; they are not worth a page.
const MAX_INTERFACES: usize = 32;

/// What sysfs says about an interface. Reading `speed` asks the driver, which
/// can wait for a kernel lock, so this runs on the sensor thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Link {
    /// `ethernet`, `wlan`, `bridge`, `tunnel`, `loopback` or `virtual`.
    pub kind: &'static str,
    /// The kernel's `operstate`: `up`, `down`, `unknown`, …
    pub state: String,
    /// A real adapter: its traffic is not a copy of another interface's.
    pub physical: bool,
    pub speed_mbps: Option<u32>,
    pub duplex: Option<String>,
    pub mtu: Option<u32>,
}

impl Default for Link {
    fn default() -> Self {
        Self {
            kind: "virtual",
            state: "unknown".into(),
            physical: false,
            speed_mbps: None,
            duplex: None,
            mtu: None,
        }
    }
}

pub(crate) fn read_links(root: &Path) -> BTreeMap<String, Link> {
    fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let link = read_link(&entry.path(), &name);
            (name, link)
        })
        .collect()
}

fn read_link(path: &Path, name: &str) -> Link {
    let text = |file: &str| {
        let value = fs::read_to_string(path.join(file)).ok()?;
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    };
    let physical = path.join("device").exists();
    let kind = if name == "lo" {
        "loopback"
    } else if path.join("wireless").exists() {
        "wlan"
    } else if path.join("bridge").exists() {
        "bridge"
    } else if path.join("tun_flags").exists() {
        "tunnel"
    } else if physical {
        "ethernet"
    } else {
        "virtual"
    };
    Link {
        kind,
        state: text("operstate").unwrap_or_else(|| "unknown".into()),
        physical,
        // A link that is down has no speed: `-1`, or an error on read.
        speed_mbps: text("speed")
            .and_then(|speed| speed.parse::<i64>().ok())
            .filter(|speed| (1..i64::from(u32::MAX)).contains(speed))
            .map(|speed| speed as u32),
        duplex: text("duplex").filter(|duplex| duplex != "unknown"),
        mtu: text("mtu").and_then(|mtu| mtu.parse().ok()),
    }
}

/// Turns each reading of `/proc/net/dev` into rates.
#[derive(Default)]
pub(crate) struct Tracker {
    previous: BTreeMap<String, Counters>,
}

impl Tracker {
    pub fn update(
        &mut self,
        text: &str,
        links: &BTreeMap<String, Link>,
        elapsed_secs: f64,
    ) -> Vec<Network> {
        let interfaces = netdev::parse(text);
        let mut networks: Vec<_> = interfaces
            .iter()
            .map(|(name, now)| {
                // A new interface has no earlier reading: no rate yet, rather
                // than everything since boot in one second.
                let before = self.previous.get(name).unwrap_or(now);
                network(name, before, now, links.get(name), elapsed_secs)
            })
            .collect();
        self.previous = interfaces.into_iter().collect();
        networks.sort_by(|a, b| {
            (b.physical, b.state == "up", &a.name).cmp(&(a.physical, a.state == "up", &b.name))
        });
        networks.truncate(MAX_INTERFACES);
        networks
    }
}

fn network(
    name: &str,
    before: &Counters,
    now: &Counters,
    link: Option<&Link>,
    elapsed: f64,
) -> Network {
    let fallback = Link::default();
    let link = link.unwrap_or(&fallback);
    Network {
        name: name.to_owned(),
        kind: link.kind.to_owned(),
        state: link.state.clone(),
        physical: link.physical,
        speed_mbps: link.speed_mbps,
        duplex: link.duplex.clone(),
        mtu: link.mtu,
        rx_bps: per_second(before.rx_bytes, now.rx_bytes, elapsed),
        tx_bps: per_second(before.tx_bytes, now.tx_bytes, elapsed),
        rx_pps: per_second(before.rx_packets, now.rx_packets, elapsed),
        tx_pps: per_second(before.tx_packets, now.tx_packets, elapsed),
        rx_total: now.rx_bytes,
        tx_total: now.tx_bytes,
        rx_errors: now.rx_errors,
        tx_errors: now.tx_errors,
        rx_drops: now.rx_drops,
        tx_drops: now.tx_drops,
    }
}

/// The sum over real adapters; bridges and veth repeat the same traffic.
pub(crate) fn physical_rates(networks: &[Network]) -> (f64, f64) {
    networks
        .iter()
        .filter(|network| network.physical)
        .fold((0.0, 0.0), |(rx, tx), network| {
            (rx + network.rx_bps, tx + network.tx_bps)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Scratch;
    use std::fmt::Write as _;

    const HEADER: &str = "Inter-|\n face |\n";

    fn dev(lines: &str) -> String {
        format!("{HEADER}{lines}")
    }

    fn links() -> BTreeMap<String, Link> {
        BTreeMap::from([
            (
                "enp5s0".to_owned(),
                Link {
                    kind: "ethernet",
                    state: "up".into(),
                    physical: true,
                    speed_mbps: Some(10_000),
                    duplex: Some("full".into()),
                    mtu: Some(1500),
                },
            ),
            (
                "docker0".to_owned(),
                Link {
                    kind: "bridge",
                    state: "down".into(),
                    ..Link::default()
                },
            ),
        ])
    }

    #[test]
    fn rates_come_from_the_difference_and_totals_from_the_counter() {
        let mut tracker = Tracker::default();
        let first = dev(concat!(
            "enp5s0: 1000 10 0 0 0 0 0 0 500 5 0 0 0 0 0 0\n",
            "docker0: 50 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n",
        ));
        let second = dev(concat!(
            "enp5s0: 3000 30 1 2 0 0 0 0 1500 15 3 4 0 0 0 0\n",
            "docker0: 150 2 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n",
        ));
        tracker.update(&first, &links(), 1.0);
        let networks = tracker.update(&second, &links(), 2.0);
        let enp = &networks[0];
        assert_eq!(enp.name, "enp5s0", "physical adapters first");
        assert!((enp.rx_bps - 1000.0).abs() < f64::EPSILON);
        assert!((enp.tx_bps - 500.0).abs() < f64::EPSILON);
        assert!((enp.rx_pps - 10.0).abs() < f64::EPSILON);
        assert_eq!(enp.rx_total, 3000);
        assert_eq!(enp.tx_total, 1500);
        assert_eq!(enp.rx_errors, 1);
        assert_eq!(enp.tx_drops, 4);
        assert_eq!(enp.speed_mbps, Some(10_000));
        let (rx, tx) = physical_rates(&networks);
        assert!(
            (rx - 1000.0).abs() < f64::EPSILON,
            "the bridge is not added"
        );
        assert!((tx - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn a_new_interface_starts_at_zero_rate_not_at_its_total() {
        let mut tracker = Tracker::default();
        let networks = tracker.update(
            &dev("veth1: 999999 9 0 0 0 0 0 0 999999 9 0 0 0 0 0 0\n"),
            &BTreeMap::new(),
            1.0,
        );
        assert!(networks[0].rx_bps.abs() < f64::EPSILON);
        assert_eq!(networks[0].rx_total, 999_999);
        assert_eq!(networks[0].kind, "virtual");
    }

    #[test]
    fn a_counter_reset_shows_no_negative_rate() {
        let mut tracker = Tracker::default();
        tracker.update(
            &dev("eth0: 9000 0 0 0 0 0 0 0 9000 0 0 0 0 0 0 0\n"),
            &BTreeMap::new(),
            1.0,
        );
        let networks = tracker.update(
            &dev("eth0: 10 0 0 0 0 0 0 0 10 0 0 0 0 0 0 0\n"),
            &BTreeMap::new(),
            1.0,
        );
        assert!(networks[0].rx_bps.abs() < f64::EPSILON);
    }

    #[test]
    fn sysfs_tells_the_kinds_apart() {
        let scratch = Scratch::new("links");
        scratch.write("net/lo/operstate", "unknown\n");
        scratch.write("net/enp5s0/operstate", "up\n");
        scratch.write("net/enp5s0/speed", "10000\n");
        scratch.write("net/enp5s0/duplex", "full\n");
        scratch.write("net/enp5s0/mtu", "1500\n");
        scratch.write("net/enp5s0/device/vendor", "0x1\n");
        scratch.write("net/enp6s0/operstate", "down\n");
        scratch.write("net/enp6s0/speed", "-1\n");
        scratch.write("net/enp6s0/duplex", "unknown\n");
        scratch.write("net/enp6s0/device/vendor", "0x1\n");
        scratch.write("net/wlp8s0/wireless/x", "");
        scratch.write("net/wlp8s0/device/vendor", "0x1\n");
        scratch.write("net/docker0/bridge/x", "");
        scratch.write("net/tun0/tun_flags", "0x1\n");
        scratch.write("net/veth1/operstate", "up\n");
        let links = read_links(&scratch.path("net"));
        assert_eq!(links["lo"].kind, "loopback");
        assert_eq!(links["enp5s0"].kind, "ethernet");
        assert!(links["enp5s0"].physical);
        assert_eq!(links["enp5s0"].speed_mbps, Some(10_000));
        assert_eq!(links["enp5s0"].duplex.as_deref(), Some("full"));
        assert_eq!(links["enp5s0"].mtu, Some(1500));
        assert_eq!(links["enp6s0"].speed_mbps, None, "-1 means no link");
        assert_eq!(links["enp6s0"].duplex, None);
        assert_eq!(links["wlp8s0"].kind, "wlan");
        assert!(links["wlp8s0"].physical);
        assert_eq!(links["docker0"].kind, "bridge");
        assert_eq!(links["tun0"].kind, "tunnel");
        assert_eq!(links["veth1"].kind, "virtual");
        assert!(!links["veth1"].physical);
    }

    #[test]
    fn a_page_of_veth_pairs_is_cut_after_the_real_adapters() {
        let mut lines = String::new();
        for n in 0..50 {
            writeln!(lines, "veth{n}: 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0").unwrap();
        }
        lines.push_str("enp5s0: 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n");
        let networks = Tracker::default().update(&dev(&lines), &links(), 1.0);
        assert_eq!(networks.len(), MAX_INTERFACES);
        assert_eq!(networks[0].name, "enp5s0");
    }
}
