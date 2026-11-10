//! The results of `ladder4` roots: which games were played, and which of them clean.
use super::read_json;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The seat's closing summary of a played game: the last line of its
/// transcript, which names what the model and the house answered.
fn seat_summary(root: &Path, game: u64) -> Result<Option<Value>> {
    let directory = root.join(format!("game-{game:04}/transcripts"));
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Ok(None);
    };
    for entry in entries {
        let path = entry?.path();
        if path.to_string_lossy().ends_with("seat1.jsonl") {
            let text = std::fs::read_to_string(path)?;
            let Some(last) = text.lines().next_back() else {
                continue;
            };
            let value: Value = serde_json::from_str(last)?;
            return Ok(Some(value["summary"].clone()));
        }
    }
    Ok(None)
}

/// Every game of `roots`, by arm: lost before it began, played clean, or played
/// with the house finishing for a model out of budget. The tally of an arm is in
/// the order of `columns`.
///
/// # Errors
/// Rejects a root without a readable `ladder.json` or a played game without a number.
pub fn ladder_table(roots: &[PathBuf]) -> Result<Value> {
    let mut arms = BTreeMap::<String, [u64; 7]>::new();
    let mut games = Vec::new();
    for root in roots {
        let report = read_json(&root.join("ladder.json"))?;
        for game in report["games"].as_array().context("no games")? {
            let key = format!(
                "{} / {}",
                game["profile"].as_str().unwrap_or(""),
                game["mode"].as_str().unwrap_or("")
            );
            let tally = arms.entry(key).or_default();
            tally[0] += 1;
            let Some(won) = game["won"].as_bool() else {
                tally[1] += 1;
                games.push(json!({"root": root, "game": game["game"], "played": false,
                    "profile": game["profile"], "mode": game["mode"],
                    "why": game["error"].as_str().map_or("timed out, no seat", |e| e)}));
                continue;
            };
            let number = game["game"].as_u64().context("game number")?;
            let summary = seat_summary(root, number)?.unwrap_or(Value::Null);
            let declined = summary["fallbacks"]["declined"].as_u64().unwrap_or(0);
            // One declined answer is a stumble; a game the house finished is not the model's.
            let model = summary["answered"]["mind"].as_u64().unwrap_or(0);
            let house = summary["answered"]["house"].as_u64().unwrap_or(0);
            // A seat that left no summary died before the end: the table's result
            // says nothing of a game the model did not finish.
            let finished = summary.is_null() || house * 20 > model + house;
            let (played, wins) = if finished { (4, 5) } else { (2, 3) };
            tally[played] += 1;
            tally[wins] += u64::from(won);
            tally[6] += house;
            games.push(json!({"root": root, "game": number, "played": true,
                "profile": game["profile"], "mode": game["mode"], "net_won": won,
                "drawn": game["drawn"],
                "model_answers": model,
                "house_answers": house, "declined": declined, "house_finished": finished,
                "turns": summary["turns"], "net_deck": game["net_deck"], "llm_deck": game["llm_deck"]}));
        }
    }
    Ok(json!({"arms": arms, "games": games,
        "columns": ["scheduled", "lost_before_play", "clean", "clean_net_wins", "house_finished",
            "house_finished_net_wins", "house_answers"]}))
}

/// The net's score in a game of a [`ladder_table`] when the game is clean: played
/// to its end by the model, the house finishing nothing. A draw counts one half,
/// as everywhere else in the evaluation. `None` for every other game.
pub fn clean_score(game: &Value) -> Option<f64> {
    if game["played"] != true || game["house_finished"] != false {
        return None;
    }
    Some(if game["net_won"] == true {
        1.0
    } else if game["drawn"] == true {
        0.5
    } else {
        0.0
    })
}
