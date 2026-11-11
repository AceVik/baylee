//! The results of `ladder4` roots: which games were played, and which of them clean.
use super::read_json;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The model seat's transcripts of a played game: the seat's own
/// (`<room>-seat<N>.jsonl`, whichever chair it sat in: chair 1 against the
/// net, chair 0 against the house) and, if `mind`, its model's calls beside
/// it (`…-mind.jsonl`). A ladder game has one model seat.
fn seat_transcripts(root: &Path, game: u64, mind: bool) -> Result<Vec<PathBuf>> {
    let directory = root.join(format!("game-{game:04}/transcripts"));
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Ok(Vec::new());
    };
    let mut found = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        let Some(name) = name else { continue };
        let lines = path.extension().is_some_and(|ext| ext == "jsonl");
        if lines && name.contains("-seat") && name.ends_with("-mind.jsonl") == mind {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

/// The seat's closing summary of a played game: the last line of its
/// transcript, which names what the model and the house answered.
fn seat_summary(root: &Path, game: u64) -> Result<Option<Value>> {
    for path in seat_transcripts(root, game, false)? {
        let text = std::fs::read_to_string(path)?;
        let Some(last) = text.lines().next_back() else {
            continue;
        };
        let value: Value = serde_json::from_str(last)?;
        return Ok(Some(value["summary"].clone()));
    }
    Ok(None)
}

/// The kinds of token a model's calls are billed by, as its transcript
/// counts them: `input` read uncached, `cache_read`, `cache_write` (five
/// minutes and an hour together) and `output`, reasoning included.
pub const TOKEN_KINDS: [&str; 4] = ["input", "cache_read", "cache_write", "output"];

/// What the model's calls in a played game took, by [`TOKEN_KINDS`]: the sum
/// of each reply's `usage` in its `-mind` transcript.
fn mind_tokens(root: &Path, game: u64) -> Result<[u64; 4]> {
    let mut sums = [0_u64; 4];
    for path in seat_transcripts(root, game, true)? {
        for line in std::fs::read_to_string(path)?.lines() {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let usage = &value["reply"]["usage"];
            let count = |field: &str| usage[field].as_u64().unwrap_or(0);
            sums[0] += count("input");
            sums[1] += count("cache_read");
            sums[2] += count("cache_write") + count("cache_write_hour");
            sums[3] += count("output");
        }
    }
    Ok(sums)
}

/// Every game of `roots`, by arm: lost before it began, played clean, or played
/// with the house finishing for a model out of budget. The tally of an arm is in
/// the order of `columns`; `tokens` sums its model's calls by [`TOKEN_KINDS`],
/// a game that never began counting what it spent.
///
/// # Errors
/// Rejects a root without a readable `ladder.json` or a played game without a number.
pub fn ladder_table(roots: &[PathBuf]) -> Result<Value> {
    let mut arms = BTreeMap::<String, [u64; 7]>::new();
    let mut tokens = BTreeMap::<String, [u64; 4]>::new();
    let mut games = Vec::new();
    for root in roots {
        let report = read_json(&root.join("ladder.json"))?;
        for game in report["games"].as_array().context("no games")? {
            let key = format!(
                "{} / {}",
                game["profile"].as_str().unwrap_or(""),
                game["mode"].as_str().unwrap_or("")
            );
            let tally = arms.entry(key.clone()).or_default();
            tally[0] += 1;
            // A game that began has its model's calls on the book whatever its
            // end: what the arm cost counts them all.
            let spent = match game["game"].as_u64() {
                Some(number) => mind_tokens(root, number)?,
                None => [0; 4],
            };
            for (sum, kind) in tokens.entry(key).or_default().iter_mut().zip(spent) {
                *sum += kind;
            }
            let Some(won) = game["won"].as_bool() else {
                tally[1] += 1;
                let why = game["error"]
                    .as_str()
                    .or_else(|| game["failure"].as_str())
                    .unwrap_or("timed out, no seat");
                games.push(json!({"root": root, "game": game["game"], "played": false,
                    "profile": game["profile"], "mode": game["mode"], "why": why,
                    "tokens": spent}));
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
                "turns": summary["turns"], "net_deck": game["net_deck"], "llm_deck": game["llm_deck"],
                "tokens": spent}));
        }
    }
    Ok(
        json!({"arms": arms, "games": games, "tokens": tokens, "token_kinds": TOKEN_KINDS,
        "columns": ["scheduled", "lost_before_play", "clean", "clean_net_wins", "house_finished",
            "house_finished_net_wins", "house_answers"]}),
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::atomic_json;

    /// A house arm's model sits in chair 0: its summary is read there all the
    /// same, and the arm's tokens are its calls' usage, a game that never
    /// began counting what it spent.
    #[test]
    fn a_ladder_reads_the_model_seat_in_either_chair_and_sums_its_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let transcripts = root.join("game-0000/transcripts");
        std::fs::create_dir_all(&transcripts).unwrap();
        let summary = json!({"summary": {"answered": {"mind": 40, "house": 0}, "turns": 9,
            "fallbacks": {"declined": 0}, "outcome": "lost"}});
        std::fs::write(
            transcripts.join("r-seat0.jsonl"),
            format!("{{\"event\":\"over\"}}\n{summary}\n"),
        )
        .unwrap();
        let call = |input: u64, read: u64, output: u64| {
            json!({"reply": {"usage": {"input": input, "cache_read": read, "cache_write": 0,
                "cache_write_hour": 5, "output": output}}})
        };
        std::fs::write(
            transcripts.join("r-seat0-mind.jsonl"),
            format!(
                "{}\n{}\n{{\"error\":\"x\"}}\n",
                call(10, 100, 3),
                call(20, 200, 4)
            ),
        )
        .unwrap();
        atomic_json(
            &root.join("ladder.json"),
            &json!({"games": [
                {"game": 0, "profile": "deepseek-low", "mode": "house=expert", "won": true,
                    "drawn": false, "net_deck": "a", "llm_deck": "b"},
                {"game": 1, "profile": "deepseek-low", "mode": "house=expert",
                    "error": "the game never began: no chair"},
            ]}),
        )
        .unwrap();
        let table = ladder_table(&[root.to_path_buf()]).unwrap();
        // scheduled, lost before play, clean, clean wins, house finished,
        // wins there, house answers.
        assert_eq!(
            table["arms"]["deepseek-low / house=expert"],
            json!([2, 1, 1, 1, 0, 0, 0])
        );
        assert_eq!(
            table["tokens"]["deepseek-low / house=expert"],
            json!([30, 300, 10, 7])
        );
        assert_eq!(table["games"][1]["why"], "the game never began: no chair");
    }
}
