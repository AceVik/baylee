//! What a game is in an `eval4` summary, for every reader that counts them.
//!
//! eval4 plays one deal per focal seat: deal `d` is table `d / seats`, focal
//! seat `d % seats`, seed `first_seed + table`, and the table's decks are
//! fixed. The net controls the focal seat and, in a team format, every seat
//! of its team. In 1v1 and free-for-all each focal seat is a game of its own,
//! since the net holds a different seat. In a team format the focal seats of
//! one team replay one game: the same seed, decks and seats under the net.
//! They differ only where something follows the focal seat itself: the net's
//! policy is seeded per deal, so they replay exactly at temperature 0 (1
//! table of 64 split in the 2v2 runs of 10 November 2026) and are draws of the
//! net's play on one deal at a sampling temperature. A game is therefore one
//! table and the side the focal seat's player controls, and a reader that
//! counts games reads [`game_rows`], never `deal_details` row by row.
//!
//! "The net" is whoever plays the focal side: in a house-against-house run
//! (no `--model`), the `--baseline` level. [`game_of`] names a game whichever
//! side a reader measures. A key names a game within one run; across runs a
//! reader adds what the runs changed: the seed for pooled splits (as
//! [`game_rows`] does), the seating for runs with the levels swapped.
// Counts of rows, well below f64's exact 2^53 range.
#![allow(clippy::cast_precision_loss)]
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// The side the net controls with `focal` as its focal seat: its team, or the
/// seat alone where there are no teams.
pub fn side(focal: usize, teams: &[Option<u8>]) -> String {
    match teams.get(focal).copied().flatten() {
        Some(team) => format!("team{team}"),
        None => format!("seat{focal}"),
    }
}

/// A game's key within one summary: its table and the net's side,
/// `"{table}/{side}"`. Tables of two summaries share numbers, so a key across
/// summaries also needs the table's seed.
pub fn game_key(table: u64, side: &str) -> String {
    format!("{table}/{side}")
}

/// The game a `deal_details` row belongs to: the `game` eval4 wrote, or, in a
/// summary from before it wrote one, its table and focal seat under the
/// summary's `teams`.
///
/// # Errors
/// Rejects a row with neither a `game` nor a table and a focal seat.
pub fn game_of(row: &Value, teams: &Value) -> Result<String> {
    if let Some(game) = row["game"].as_str() {
        return Ok(game.to_owned());
    }
    let table = row["table"].as_u64().context("a row without a table")?;
    let focal = usize::try_from(
        row["focal_seat"]
            .as_u64()
            .context("a row without a focal seat")?,
    )?;
    let teams: Vec<Option<u8>> = teams
        .as_array()
        .into_iter()
        .flatten()
        // eval4 writes `--teams` as given: 0 is a seat on its own.
        .map(|team| {
            team.as_u64()
                .and_then(|team| u8::try_from(team).ok())
                .filter(|&team| team != 0)
        })
        .collect();
    Ok(game_key(table, &side(focal, &teams)))
}

/// The mean of the rows' `key`, or null when one has none.
fn mean(rows: &[&Value], key: &str) -> Value {
    let values: Option<Vec<f64>> = rows.iter().map(|row| row[key].as_f64()).collect();
    values.map_or(Value::Null, |values| {
        json!(values.iter().sum::<f64>() / values.len() as f64)
    })
}

/// A summary's rows, one per game: the rows of a game made one, its
/// utilities their mean, capped where any row was, with `game` its key,
/// `rows` how many rows it had and `split` whether their outcomes differed.
/// Every other field is its first row's. In 1v1 every row is a game and comes
/// back as it was, with its `game` key.
///
/// Rows of one key under different seeds are different games: the rows of
/// two splits pooled into one list share table numbers.
///
/// # Errors
/// Rejects a summary without `deal_details`, a row [`game_of`] cannot place,
/// and rows of one game dealt different decks.
pub fn game_rows(summary: &Value) -> Result<Vec<Value>> {
    let mut order = Vec::new();
    let mut grouped = BTreeMap::<(String, String), Vec<&Value>>::new();
    for row in summary["deal_details"]
        .as_array()
        .context("missing deal_details")?
    {
        let key = (row["seed"].to_string(), game_of(row, &summary["teams"])?);
        let rows = grouped.entry(key.clone()).or_default();
        if rows.is_empty() {
            order.push(key);
        }
        rows.push(row);
    }
    order
        .into_iter()
        .map(|grouping| {
            let rows = &grouped[&grouping];
            let key = grouping.1;
            let first = rows[0];
            ensure!(
                rows.iter().all(|row| row["decks"] == first["decks"]),
                "the rows of game {key} were dealt different decks"
            );
            let mut game = first.clone();
            game["net_utility"] = mean(rows, "net_utility");
            game["base_utility"] = mean(rows, "base_utility");
            for capped in ["net_capped", "base_capped"] {
                if first.get(capped).is_some() {
                    game[capped] = json!(rows.iter().any(|row| row[capped] == true));
                }
            }
            game["game"] = json!(key);
            game["rows"] = json!(rows.len());
            game["split"] = json!(
                rows.iter()
                    .any(|row| row["net_utility"] != first["net_utility"])
            );
            Ok(game)
        })
        .collect()
}

/// The counts eval4 writes beside its row counts: games, and of those with an
/// outcome the ones won, drawn (a split game among them), lost and stopped at
/// the answer cap, with the games that split and their mean utility.
///
/// # Errors
/// As [`game_rows`].
pub fn game_counts(summary: &Value) -> Result<Value> {
    let games = game_rows(summary)?;
    let (mut wins, mut draws, mut losses, mut caps, mut splits) = (0, 0, 0, 0, 0);
    let (mut sum, mut scored) = (0.0, 0_usize);
    for game in &games {
        splits += usize::from(game["split"] == true);
        let Some(utility) = game["net_utility"].as_f64() else {
            continue;
        };
        sum += utility;
        scored += 1;
        if game["net_capped"] == true {
            caps += 1;
        } else if utility >= 0.99 {
            wins += 1;
        } else if utility <= 0.01 {
            losses += 1;
        } else {
            draws += 1;
        }
    }
    Ok(
        json!({"games": games.len(), "game_wins": wins, "game_draws": draws,
        "game_losses": losses, "game_answer_caps": caps, "game_splits": splits,
        "game_utility_mean": (scored > 0).then(|| sum / scored as f64)}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(table: u64, focal: u64, utility: f64) -> Value {
        json!({"table": table, "seed": 100 + table, "focal_seat": focal,
            "decks": ["a", "b", "c", "d"], "net_utility": utility, "net_capped": false,
            "base_utility": null, "base_capped": false})
    }

    #[test]
    fn a_team_game_is_counted_once_however_many_seats_replayed_it() {
        // Two 2v2 tables, four rows each: seats 0 and 2 one team, 1 and 3 the other.
        let summary = json!({"seats": 4, "teams": [1, 2, 1, 2], "deal_details": [
            row(0, 0, 1.0), row(0, 1, 0.0), row(0, 2, 1.0), row(0, 3, 0.0),
            row(1, 0, 1.0), row(1, 1, 0.0), row(1, 2, 0.0), row(1, 3, 1.0)]});
        let games = game_rows(&summary).unwrap();
        let keys: Vec<&str> = games.iter().map(|g| g["game"].as_str().unwrap()).collect();
        assert_eq!(keys, ["0/team1", "0/team2", "1/team1", "1/team2"]);
        assert!(games.iter().all(|game| game["rows"] == 2));
        assert_eq!(games[2]["net_utility"], 0.5, "a split game scores its mean");
        assert_eq!(games[2]["split"], true);
        assert_eq!(
            game_counts(&summary).unwrap(),
            json!({"games": 4, "game_wins": 1, "game_draws": 2, "game_losses": 1,
                "game_answer_caps": 0, "game_splits": 2, "game_utility_mean": 0.5})
        );
    }

    #[test]
    fn in_one_v_one_every_row_is_a_game() {
        let mut rows = vec![
            row(0, 0, 1.0),
            row(0, 1, 1.0),
            row(1, 0, 0.0),
            row(1, 1, 0.5),
        ];
        rows[3]["net_capped"] = json!(true);
        let summary = json!({"seats": 2, "teams": [], "deal_details": rows});
        let games = game_rows(&summary).unwrap();
        assert_eq!(games.len(), 4);
        for (game, row) in games
            .iter()
            .zip(summary["deal_details"].as_array().unwrap())
        {
            let mut game = game.clone();
            for key in ["game", "rows", "split"] {
                game.as_object_mut().unwrap().remove(key);
            }
            assert_eq!(&game, row, "a 1v1 row comes back as it was");
        }
        assert_eq!(games[1]["game"], "0/seat1");
        assert_eq!(
            game_counts(&summary).unwrap(),
            json!({"games": 4, "game_wins": 2, "game_draws": 0, "game_losses": 1,
                "game_answer_caps": 1, "game_splits": 0, "game_utility_mean": 0.625})
        );
    }

    #[test]
    fn a_written_game_key_is_read_as_it_is() {
        let keyed = |focal: u64, seed: u64| {
            let mut row = row(0, focal, 1.0);
            row["game"] = json!("0/team1");
            row["seed"] = json!(seed);
            row
        };
        // The key eval4 wrote wins over the table and focal seat.
        let summary = json!({"teams": [], "deal_details": [keyed(0, 100), keyed(1, 100)]});
        assert_eq!(game_rows(&summary).unwrap().len(), 1);
        // One key under two seeds is two games: two splits pooled.
        let pooled = json!({"teams": [], "deal_details": [keyed(0, 100), keyed(1, 7)]});
        assert_eq!(game_rows(&pooled).unwrap().len(), 2);
        let mut other = keyed(1, 100);
        other["decks"] = json!(["e", "f"]);
        let mismatched = json!({"teams": [], "deal_details": [keyed(0, 100), other]});
        assert!(game_rows(&mismatched).is_err(), "one game, two deck sets");
    }

    #[test]
    fn a_seat_on_its_own_is_its_own_side() {
        let row = row(0, 0, 1.0);
        assert_eq!(game_of(&row, &json!([0, 1, 1])).unwrap(), "0/seat0");
        assert_eq!(game_of(&row, &json!([2, 1, 1])).unwrap(), "0/team2");
        assert_eq!(game_of(&row, &json!([])).unwrap(), "0/seat0");
    }
}
