//! Every training conversion's deck guard (`--train-manifest`): before any
//! game is replayed, the decks its record's header seats are read against the
//! manifest (`baylee_train::decksplit::TrainOnly`: by key, exact cards or
//! near), and a game that seated a deck outside `train` (test, val, holdout2)
//! stops the conversion, named with the deck. One check for every mode:
//! on-policy, shadow, LLM and live BC, self-imitation, search distillation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use baylee_train::decksplit::{Cards, Seat, TrainOnly, record_seats, run_deck_keys};
use serde_json::Value;

use super::GameRef;

/// The keys a game's `games.jsonl` row gives its seats: a list of keys (v5
/// teacher runs) or of objects with a `key` (converted language-model runs).
pub(super) fn row_keys(row: &Value) -> Vec<Option<String>> {
    row["decks"]
        .as_array()
        .map(|decks| {
            decks
                .iter()
                .map(|deck| {
                    deck.as_str()
                        .or_else(|| deck["key"].as_str())
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The run a game's shard belongs to (`RUN/records/SHARD`).
fn run_of(shard: &Path) -> Option<&Path> {
    shard.parent().and_then(Path::parent)
}

/// Why each of `games` may not be trained on, if any may not.
///
/// # Errors
/// A record whose header cannot be read.
pub(super) fn refusals(guard: &TrainOnly, games: &[GameRef]) -> anyhow::Result<Vec<String>> {
    let mut named: BTreeMap<PathBuf, BTreeMap<Cards, String>> = BTreeMap::new();
    let mut refused = Vec::new();
    for game in games {
        let seats = record_seats(&game.shard_path, game.offset, game.len)?;
        let run = run_of(&game.shard_path).unwrap_or(Path::new(""));
        let keys = named.entry(run.to_path_buf()).or_insert_with(|| {
            std::fs::read(run.join("run.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .map(|run_json| run_deck_keys(&run_json))
                .unwrap_or_default()
        });
        let seats: Vec<Seat> = seats
            .into_iter()
            .enumerate()
            .map(|(index, cards)| Seat {
                key: game
                    .deck_keys
                    .get(index)
                    .cloned()
                    .flatten()
                    .or_else(|| keys.get(&cards).cloned()),
                cards: Some(cards),
            })
            .collect();
        if let Some(why) = guard.refusal(&seats) {
            refused.push(format!(
                "{} at {} (game {}): {why}",
                game.shard_path.display(),
                game.offset,
                game.game_id
            ));
        }
    }
    Ok(refused)
}

/// Refuses the conversion when any game seated a deck outside `train`.
///
/// # Errors
/// As [`refusals`], and the refused games, the first twenty named.
pub(super) fn check(guard: &TrainOnly, games: &[GameRef]) -> anyhow::Result<()> {
    let refused = refusals(guard, games)?;
    anyhow::ensure!(
        refused.is_empty(),
        "{} of {} games seated a deck outside the train split; take their runs out:\n{}",
        refused.len(),
        games.len(),
        refused
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_train::convert4::TeamHands;
    use baylee_train::decksplit::cards_of;
    use baylee_train::housedeck::HouseDeck;
    use baylee_train::working::repo_root;
    use serde_json::json;
    use std::io::Write as _;

    fn cards(path: &str) -> Cards {
        let text = std::fs::read_to_string(repo_root().join(path)).unwrap();
        cards_of(&HouseDeck::parse_any("x", &text).unwrap())
    }

    fn game(
        shard: &Path,
        game_id: u32,
        offset: u64,
        len: u64,
        keys: Vec<Option<String>>,
    ) -> GameRef {
        GameRef {
            game_id,
            seat_weights: None,
            producer_encoder: None,
            team_hands: TeamHands::from_run(&json!({})),
            on_policy: None,
            shard_path: shard.to_path_buf(),
            offset,
            len,
            sidecar: None,
            search_events: None,
            validation: false,
            shadow: None,
            llm: None,
            deck_keys: keys,
        }
    }

    /// Two eval4 games and one language-model game: the one that seated a
    /// holdout2 deck (named by the run's deck list) and the one whose row
    /// names a test deck are refused by name; the train game passes.
    #[test]
    fn a_game_with_a_deck_outside_train_is_refused_by_name() {
        let dir = std::env::temp_dir().join(format!("train-decks-{}", std::process::id()));
        let run = dir.join("run");
        std::fs::create_dir_all(run.join("records")).unwrap();
        let azorius = cards("data/decks/eval/azorius-control.txt");
        let burn = cards("data/decks/eval/red-burn.txt");
        let blue = cards("data/decks/eval/blue-fliers.txt");
        let seat = |cards: &Cards| json!({"deck": cards.iter().map(|c| json!({"card": c})).collect::<Vec<_>>()});
        let mut shard = Vec::new();
        let mut members = Vec::new();
        for seats in [[&burn, &burn], [&burn, &azorius], [&blue, &burn]] {
            let header =
                json!({"kind": "header", "preset": {"seats": [seat(seats[0]), seat(seats[1])]}});
            let mut member = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
            writeln!(member, "{header}").unwrap();
            let member = member.finish().unwrap();
            members.push((shard.len() as u64, member.len() as u64));
            shard.extend(member);
        }
        let shard_path = run.join("records/w00.jsonl.gz");
        std::fs::write(&shard_path, shard).unwrap();
        std::fs::write(
            run.join("run.json"),
            serde_json::to_vec(&json!({"decks": [{"key": "pool/azorius", "cards": azorius}]}))
                .unwrap(),
        )
        .unwrap();
        let manifest = dir.join("manifest.json");
        std::fs::write(
            &manifest,
            serde_json::to_vec(&json!({"decks": [
                {"id": "pool/azorius", "path": "data/decks/eval/azorius-control.txt", "split": "holdout2"},
                {"id": "pool/burn", "path": "data/decks/eval/red-burn.txt", "split": "train"},
                {"id": "pool/fliers", "path": "data/decks/eval/blue-fliers.txt", "split": "test"},
            ]}))
            .unwrap(),
        )
        .unwrap();
        let guard = TrainOnly::load(&manifest, &repo_root()).unwrap();
        let games = [
            game(&shard_path, 1, members[0].0, members[0].1, Vec::new()),
            game(&shard_path, 2, members[1].0, members[1].1, Vec::new()),
            game(
                &shard_path,
                3,
                members[2].0,
                members[2].1,
                row_keys(&json!({"decks": [
                {"key": "house-007"}, {"key": "pool/burn"}]})),
            ),
        ];
        let refused = refusals(&guard, &games).unwrap();
        assert_eq!(refused.len(), 2, "{refused:?}");
        assert!(refused[0].contains("(game 2)") && refused[0].contains("pool/azorius"));
        assert!(refused[0].contains("holdout2"));
        assert!(refused[1].contains("(game 3)") && refused[1].contains("pool/fliers"));
        let error = check(&guard, &games).unwrap_err().to_string();
        assert!(error.contains("2 of 3 games"), "{error}");
        assert!(check(&guard, &games[..1]).is_ok());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
