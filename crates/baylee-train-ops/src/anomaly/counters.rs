//! The counterspell watch: what the net's seat countered, over every game
//! of a run that kept records (`gamelog4 counters`, which replays them
//! with the engine; this crate has none). A regression watch for a net
//! that reacts more: the share of its counterspells aimed at a poor
//! target (its own or a teammate's spell, or one of mana value 1 or less).

use std::path::Path;
use std::process::Command;

use serde_json::Value;

use super::{Class, Entry, Kind};

/// A run whose counterspells go to poor targets more often than this is
/// worth a look.
pub const POOR_SHARE: f64 = 0.25;

/// The watch over `records` with `gamelog4`: its JSON (or why there is
/// none) and the entry it makes.
#[must_use]
pub fn watch(records: &Path, gamelog4: &Path) -> (Value, Vec<Entry>) {
    let output = Command::new(gamelog4)
        .args(["counters", "--threads", "1", "--run"])
        .arg(records)
        .output();
    let output = match output {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let why = if stderr.contains("unrecognized subcommand") {
                format!("{} has no `counters` (an older build)", gamelog4.display())
            } else {
                format!(
                    "gamelog4 counters failed: {}",
                    stderr.lines().last().unwrap_or_default()
                )
            };
            return (Value::String(why), Vec::new());
        }
        Err(error) => {
            return (
                Value::String(format!("{}: {error}", gamelog4.display())),
                Vec::new(),
            );
        }
    };
    let Ok(counted) = serde_json::from_slice::<Value>(&output.stdout) else {
        return (
            Value::String("gamelog4 counters wrote no JSON".to_owned()),
            Vec::new(),
        );
    };
    let entries = entries(&counted);
    (counted, entries)
}

/// The entry a count makes: one for the run, `new` above [`POOR_SHARE`].
#[must_use]
pub fn entries(counted: &Value) -> Vec<Entry> {
    let total = counted["counterspells"].as_u64().unwrap_or(0);
    if total == 0 {
        return Vec::new();
    }
    let poor = counted["poor"].as_u64().unwrap_or(0);
    let share = counted["poor_share"].as_f64().unwrap_or(0.0);
    let class = if share > POOR_SHARE {
        Class::New
    } else {
        Class::Expected
    };
    let what = format!(
        "{poor} of {total} counterspells of the net's seat on a poor target ({:.0}%: own or a \
         teammate's spell, or mana value {} or less); by whose {}, by kind {}",
        share * 100.0,
        counted["cheap_at_most"],
        counted["by_whose"],
        counted["by_kind"]
    );
    vec![Entry::new(Kind::CounterTargets, class, what).times(poor)]
}
