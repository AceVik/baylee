//! A run directory (`games.jsonl`): selfplay's games, or the records an
//! eval4 run kept with `--record-out`.

use std::path::Path;

use serde_json::Value;

use super::{Class, Entry, Kind, Report};

/// The anomalies a run's `games.jsonl` holds, one game per line.
#[must_use]
pub fn from_run(games: &str, run_json: Option<&Value>, result: &Path) -> Report {
    let mut report = Report::new(result, "run");
    report.build = run_json.and_then(|run| {
        run["evaluator_build"]
            .as_str()
            .or_else(|| run["evaluator_revision"].as_str())
            .or_else(|| run["build"].as_str())
            .map(str::to_owned)
    });
    let (mut refused, mut fallbacks) = (0_u64, 0_u64);
    for line in games.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(game) = serde_json::from_str::<Value>(line) else {
            report.entries.push(Entry::new(
                Kind::NoEnd,
                Class::New,
                "an unreadable games.jsonl line",
            ));
            continue;
        };
        report.games += 1;
        let deal = game["i"].as_u64();
        let seed = game["seed"].as_u64();
        let seat = game["focal_seat"].as_u64();
        let place = |entry: Entry| entry.at(deal, seed, None).seat(seat);
        let outcome = &game["outcome"];
        match outcome["kind"].as_str().unwrap_or("") {
            "answer_cap" => report.entries.push(place(Entry::new(
                Kind::AnswerCap,
                Class::New,
                "the game hit the answer cap",
            ))),
            "time_cap" => report.entries.push(place(Entry::new(
                Kind::NoEnd,
                Class::New,
                "the game hit the wall-time cap",
            ))),
            "panicked" => {
                // The action in flight is in the run's fault bundle, not here.
                let what = format!(
                    "the host panicked: {}",
                    outcome["message"].as_str().unwrap_or("?")
                );
                report
                    .entries
                    .push(place(Entry::new(Kind::Panic, Class::New, what)));
            }
            _ => {}
        }
        if game["capped"].as_bool() == Some(true)
            && !matches!(outcome["kind"].as_str(), Some("answer_cap" | "time_cap"))
        {
            report.entries.push(place(Entry::new(
                Kind::AnswerCap,
                Class::New,
                "the game was capped",
            )));
        }
        if game["complete"].as_bool() == Some(false) {
            report.entries.push(place(Entry::new(
                Kind::NoEnd,
                Class::New,
                "the record stops short of the game's end",
            )));
        }
        refused += game["refused_answers"].as_u64().unwrap_or(0);
        fallbacks += game["fallback_answers"].as_u64().unwrap_or(0);
        for event in game["fallback_events"].as_array().into_iter().flatten() {
            if event["reason"].as_str() == Some("engine_refusal") {
                let what = format!(
                    "the engine refused {} (decision {}): {}",
                    event["action"], event["decision_seq"], event["error"]
                );
                report.entries.push(
                    place(Entry::new(Kind::EngineRefusal, Class::New, what))
                        .seat(event["seat"].as_u64()),
                );
            }
        }
    }
    report
        .totals
        .insert("refused_answers".to_owned(), refused.into());
    report
        .totals
        .insert("fallback_answers".to_owned(), fallbacks.into());
    report.recount();
    report
}
