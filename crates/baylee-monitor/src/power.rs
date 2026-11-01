//! Read existing package-energy counters without changing their permissions.

use std::{fs, path::PathBuf, time::Instant};

struct Package {
    path: PathBuf,
    range: u64,
    previous: Option<(u64, Instant)>,
}

pub(crate) struct Power {
    packages: Vec<Package>,
}

impl Power {
    pub fn new() -> Self {
        let packages = fs::read_dir("/sys/class/powercap")
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                let name = fs::read_to_string(path.join("name")).ok()?;
                // Only package domains: adding their core subdomains double-counts.
                if !name.trim().starts_with("package-") {
                    return None;
                }
                Some(Package {
                    range: number(&path.join("max_energy_range_uj"))?,
                    path: path.join("energy_uj"),
                    previous: None,
                })
            })
            .collect();
        Self { packages }
    }

    #[allow(clippy::cast_precision_loss)] // Joules per second need only sensor-level precision.
    pub fn sample(&mut self) -> (Option<f64>, String) {
        if self.packages.is_empty() {
            return (None, "Kein CPU-Package-Energiezähler vorhanden".into());
        }
        let mut watts = 0.0;
        let mut complete = true;
        let mut readable = true;
        for package in &mut self.packages {
            let now = Instant::now();
            if let Some(energy) = number(&package.path) {
                let previous = package.previous.replace((energy, now));
                if let Some((last, at)) = previous {
                    watts += energy_delta(last, energy, package.range) as f64
                        / 1_000_000.0
                        / now.duration_since(at).as_secs_f64();
                } else {
                    complete = false;
                }
            } else {
                package.previous = None;
                complete = false;
                readable = false;
            }
        }
        let status = if !readable {
            "CPU-Energiezähler nicht lesbar (Berechtigung oder Sensor)"
        } else if complete {
            "CPU-Package aus gemessener Energie; ohne restlichen PC"
        } else {
            "CPU-Energiezähler wird eingemessen"
        };
        (complete.then_some(watts), status.into())
    }
}

fn number(path: &std::path::Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn energy_delta(last: u64, current: u64, range: u64) -> u64 {
    if current >= last {
        current - last
    } else {
        range.saturating_sub(last).saturating_add(current)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn package_energy_handles_counter_wrap() {
        assert_eq!(super::energy_delta(990, 10, 1000), 20);
        assert_eq!(super::energy_delta(10, 30, 1000), 20);
    }
}
