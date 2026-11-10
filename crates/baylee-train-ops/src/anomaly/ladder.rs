//! A ladder4 root (`ladder.json` and a `game-NNNN` directory per game):
//! seats that failed or ran out of time, games with no end, and the panics
//! in the seat logs.

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{Class, Entry, Kind, Report};

/// Whether a seat's failure is the gateway's sign-in limit (30 guests per
/// address a day, `docs/training-runbook.md`): known, and no bug.
fn sign_in_limit(text: &str) -> bool {
    text.contains("429") || text.contains("too many attempts")
}

/// The anomalies of a ladder4 root.
#[must_use]
pub fn from_ladder(ladder: &Value, root: &Path) -> Report {
    let mut report = Report::new(root, "ladder4");
    if let Some(summary) = ladder.get("summary") {
        report.totals.insert("summary".to_owned(), summary.clone());
    }
    for game in ladder["games"].as_array().into_iter().flatten() {
        report.games += 1;
        let index = game["game"].as_u64();
        let place = |entry: Entry| entry.at(index, None, None);
        let logs = index.map_or_else(String::new, |i| {
            seat_logs(&root.join(format!("game-{i:04}")))
        });
        if let Some(error) = game["error"].as_str() {
            let known = sign_in_limit(error) || sign_in_limit(&logs);
            let class = if known { Class::Known } else { Class::New };
            let what = if known {
                format!("{error} (the gateway's sign-in limit)")
            } else {
                error.to_owned()
            };
            report
                .entries
                .push(place(Entry::new(Kind::SeatError, class, what)));
        }
        if game["timed_out"].as_bool() == Some(true) {
            let what = format!("the game ran out of time after {}s", game["seconds"]);
            report
                .entries
                .push(place(Entry::new(Kind::SeatTimeout, Class::New, what)));
        }
        if let Some(failure) = game.get("failure").filter(|f| !f.is_null()) {
            report.entries.push(place(Entry::new(
                Kind::SeatError,
                Class::New,
                format!("the seat reported a failure: {failure}"),
            )));
        }
        if game.get("error").is_none()
            && game["timed_out"].as_bool() != Some(true)
            && game["phase"].as_str() != Some("game-complete")
        {
            let what = format!("the game has no end (phase {})", game["phase"]);
            report
                .entries
                .push(place(Entry::new(Kind::NoEnd, Class::New, what)));
        }
        for line in logs.lines().filter(|l| l.contains("panicked")) {
            report.entries.push(place(Entry::new(
                Kind::Panic,
                Class::New,
                line.trim().to_owned(),
            )));
        }
    }
    report.not_recorded.extend([
        "a ladder game has no seed or deal: it is hosted by a gateway, and its record is in that \
         gateway's store"
            .to_owned(),
        "a seat's answers that the engine refused are not counted in ladder.json".to_owned(),
    ]);
    report.recount();
    report
}

/// The seat logs of one game, as one text.
fn seat_logs(dir: &Path) -> String {
    ["seat.stderr.log", "seat4.stderr.log"]
        .iter()
        .filter_map(|name| fs::read_to_string(dir.join(name)).ok())
        .collect::<Vec<_>>()
        .join("\n")
}
