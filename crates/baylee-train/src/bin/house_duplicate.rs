//! How often one house level beats another on the same deals, from eval4
//! runs of the house against the house, each game counted once.
//!
//! ```text
//! eval4 --random-policy --baseline sharp --against sharp-2026-11 ... --out a.json
//! eval4 --random-policy --baseline sharp-2026-11 --against sharp ... --out b.json
//! cargo run -p baylee-train --features play --bin house_duplicate -- --run a.json --run b.json
//! ```
//!
//! With `--random-policy` the net's game is noise and only eval4's baseline
//! game is read: its `--baseline` level plays the focal side, its
//! `--against` level every other seat. eval4 rotates the focal seat over a
//! deal's seats with one seed and one set of decks, so the rows of a run
//! repeat a game once per seat of a side (twice in 2v2, three times in 3v3),
//! and a swapped run repeats them again. A game here is a seed and the seats
//! the measured level sits in; every row of one game must agree (the
//! engine and the house are deterministic), and it is counted once.
//!
//! Each seed is played from both sides, so its luck (decks, draws, seats)
//! cancels. Reports the measured level's share of decided games with a
//! Wilson 95 % interval, the seeds by how many of their sides it won, the
//! two-sided sign test over the seeds it turned (won from every side, or
//! lost from every side), which is what the ladder's gate reads, the
//! per-deck rule (no deck of ten seeds or more turned against more often
//! than for), and per deck its side's results with each level playing it
//! and the seeds where the level alone made the difference.
//!
//! `--before <summary>` (the same deals played by another build of the
//! level, against the same opponent) adds the comparison that reads a small
//! change: the games are the same but for the level's answers, so only the
//! games that went another way count, and the sign test is over those. A
//! build that changes a hundred of a thousand games shows at a net of twenty,
//! two points of share, where each run's own interval is six points wide.

use anyhow::{Context as _, bail, ensure};
use clap::Parser;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about = "Counts eval4 house-against-house games once, paired by seed")]
struct Args {
    /// An eval4 summary, run with `--random-policy`; give the swapped run
    /// too, or a table whose focal seat visits both sides.
    #[arg(long = "run", required = true)]
    runs: Vec<PathBuf>,
    /// The same games as another build of the level played them (its eval4
    /// summaries, same deals): also reports, game by game, how many the
    /// level now wins that it lost then and the other way round.
    #[arg(long = "before")]
    before: Vec<PathBuf>,
    /// The level measured; the first run's `--baseline` when not given.
    #[arg(long)]
    level: Option<String>,
    /// Where to write the numbers as JSON, besides the text.
    #[arg(long)]
    out: Option<PathBuf>,
}

/// One game's result for the level measured.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Result {
    Won,
    Lost,
    /// A draw or a game stopped at its cap: not decided.
    Open,
}

/// Wins and decided games.
#[derive(Default, Clone, Copy, Debug)]
struct Share {
    won: u64,
    decided: u64,
    open: u64,
}

impl Share {
    fn add(&mut self, result: Result) {
        match result {
            Result::Won => {
                self.won += 1;
                self.decided += 1;
            }
            Result::Lost => self.decided += 1,
            Result::Open => self.open += 1,
        }
    }

    fn json(self) -> Value {
        let (low, high) = wilson(self.won, self.decided);
        json!({
            "won": self.won,
            "decided": self.decided,
            "open": self.open,
            "share": rate(self.won, self.decided),
            "wilson95": [low, high],
        })
    }
}

/// One deck's games: its side's results with each level playing it, and
/// the seeds where the level alone made the difference.
#[derive(Default, Clone, Copy, Debug)]
struct Deck {
    with_level: Share,
    with_other: Share,
    turned_for: u64,
    turned_against: u64,
}

impl Deck {
    fn add(&mut self, with_level: Result, with_other: Result) {
        self.with_level.add(with_level);
        self.with_other.add(with_other);
        match (with_level, with_other) {
            (Result::Won, Result::Lost) => self.turned_for += 1,
            (Result::Lost, Result::Won) => self.turned_against += 1,
            _ => {}
        }
    }
}

#[allow(clippy::cast_precision_loss, reason = "game counts are small")]
fn rate(k: u64, n: u64) -> f64 {
    if n == 0 { 0.0 } else { k as f64 / n as f64 }
}

/// The two-sided exact sign test of `wins` against `losses`: the chance,
/// were the two levels equal, of a split at least this uneven.
///
/// The test a duplicate pairing is read by. Each seed is played from both
/// sides, and a seed the level won from one side and lost from the other
/// says nothing about the level; only the seeds it won from every side or
/// lost from every side do, and under equal levels each of those is a fair
/// coin. The Wilson interval over games treats every game as independent
/// and is far wider.
fn sign_test(wins: u64, losses: u64) -> f64 {
    let n = wins + losses;
    if n == 0 {
        return 1.0;
    }
    let tail = wins.max(losses);
    // ln C(n, i) − n ln 2 for each i ≥ tail, summed in the log domain so
    // that thousands of seeds neither underflow nor overflow.
    #[allow(clippy::cast_precision_loss, reason = "seed counts are small")]
    let ln = |x: u64| (x as f64).ln();
    let mut ln_choose = 0.0_f64;
    let mut terms = Vec::new();
    for i in 0..=n {
        if i >= tail {
            terms.push(ln_choose);
        }
        if i < n {
            ln_choose += ln(n - i) - ln(i + 1);
        }
    }
    #[allow(clippy::cast_precision_loss, reason = "seed counts are small")]
    let half = -(n as f64) * std::f64::consts::LN_2;
    let top = terms.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = terms.iter().map(|t| (t - top).exp()).sum();
    (2.0 * (top + sum.ln() + half).exp()).min(1.0)
}

/// The Wilson score interval at 95 %.
#[allow(clippy::cast_precision_loss, reason = "game counts are small")]
fn wilson(k: u64, n: u64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let z = 1.959_964_f64;
    let n = n as f64;
    let p = k as f64 / n;
    let centre = p + z * z / (2.0 * n);
    let spread = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt();
    let scale = 1.0 + z * z / n;
    ((centre - spread) / scale, (centre + spread) / scale)
}

fn read(path: &PathBuf) -> anyhow::Result<Value> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// The result of a row's baseline game for the focal side.
fn focal(row: &Value) -> Result {
    if row["base_capped"].as_bool().unwrap_or(false) {
        return Result::Open;
    }
    match row["base_utility"].as_f64() {
        Some(u) if u >= 1.0 => Result::Won,
        Some(u) if u <= 0.0 => Result::Lost,
        _ => Result::Open,
    }
}

const fn flipped(result: Result) -> Result {
    match result {
        Result::Won => Result::Lost,
        Result::Lost => Result::Won,
        Result::Open => Result::Open,
    }
}

/// The seats on the focal seat's side (its team, or itself without teams).
fn focal_side(row: &Value, teams: &[u64], seats: usize) -> Vec<usize> {
    let at = usize::try_from(row["focal_seat"].as_u64().unwrap_or(0)).unwrap_or(0);
    match teams.get(at) {
        Some(team) => (0..seats).filter(|&s| teams.get(s) == Some(team)).collect(),
        None => vec![at],
    }
}

/// A game: its seed and the seats the measured level sits in.
type Key = (u64, Vec<usize>);

/// What a game was dealt and how it went.
struct Game {
    decks: Vec<String>,
    result: Result,
}

/// Every game of `runs` once, with the level the other side played and how
/// many rows the runs held.
fn collect(
    runs: &[Value],
    level: &str,
    seats: usize,
    teams: &[u64],
) -> anyhow::Result<(String, BTreeMap<Key, Game>, u64)> {
    let mut other = String::new();
    let mut games: BTreeMap<Key, Game> = BTreeMap::new();
    let mut rows = 0_u64;
    for run in runs {
        let (baseline, against) = (
            run["baseline"].as_str().unwrap_or("?"),
            run["against"].as_str().unwrap_or("?"),
        );
        let level_is_focal = if baseline == level {
            against.clone_into(&mut other);
            true
        } else if against == level {
            baseline.clone_into(&mut other);
            false
        } else {
            bail!("a run plays neither side at {level}");
        };
        for row in run["deal_details"].as_array().into_iter().flatten() {
            rows += 1;
            let seed = row["seed"].as_u64().context("a row without a seed")?;
            let side = focal_side(row, teams, seats);
            let (seated, result) = if level_is_focal {
                (side, focal(row))
            } else {
                let rest = (0..seats).filter(|s| !side.contains(s)).collect();
                (rest, flipped(focal(row)))
            };
            let decks: Vec<String> = row["decks"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|d| d.as_str().map(str::to_owned))
                .collect();
            match games.get(&(seed, seated.clone())) {
                Some(game) => ensure!(
                    game.result == result && game.decks == decks,
                    "seed {seed} with the level at {seated:?} was played twice and went two ways"
                ),
                None => {
                    games.insert((seed, seated), Game { decks, result });
                }
            }
        }
    }
    Ok((other, games, rows))
}

/// Each seed from its sides: how many of them the level won, per deck what
/// its seat's side scored with each level in it, and the seeds it turned:
/// won from every side it played, and lost from every side.
fn tally(
    games: &BTreeMap<Key, Game>,
    seats: usize,
) -> (BTreeMap<String, u64>, BTreeMap<String, Deck>, (u64, u64)) {
    let mut by_seed: BTreeMap<u64, Vec<(&Vec<usize>, &Game)>> = BTreeMap::new();
    for ((seed, seated), game) in games {
        by_seed.entry(*seed).or_default().push((seated, game));
    }
    let mut sides = BTreeMap::<String, u64>::new();
    let mut per_deck: BTreeMap<String, Deck> = BTreeMap::new();
    let mut turned = (0, 0);
    for played in by_seed.values() {
        let label = if played.iter().any(|(_, g)| g.result == Result::Open) {
            "with an open game".to_owned()
        } else {
            let won = played
                .iter()
                .filter(|(_, g)| g.result == Result::Won)
                .count();
            if played.len() > 1 && won == played.len() {
                turned.0 += 1;
            } else if played.len() > 1 && won == 0 {
                turned.1 += 1;
            }
            format!("{won} of {}", played.len())
        };
        *sides.entry(label).or_default() += 1;
        for seat in 0..seats {
            let with_level = played.iter().find(|(seated, _)| seated.contains(&seat));
            let with_other = played.iter().find(|(seated, _)| !seated.contains(&seat));
            let (Some((_, mine)), Some((_, theirs))) = (with_level, with_other) else {
                continue;
            };
            let Some(deck) = mine.decks.get(seat) else {
                continue;
            };
            // The other level sat at this seat: its side's result is the
            // level's result flipped.
            per_deck
                .entry(deck.clone())
                .or_default()
                .add(mine.result, flipped(theirs.result));
        }
    }
    (sides, per_deck, turned)
}

/// The games both `now` and `then` played to a decision: how many the level
/// wins now that it lost then, how many it loses now that it won then, and
/// how many there are.
///
/// The games are the same deals with the same opponent, and the house is
/// deterministic, so a game goes another way only where the level answers
/// differently: under two equal builds a changed game is a fair coin, and
/// the sign test over the changed games reads a difference that the two
/// shares' intervals, which treat the runs as independent, cannot.
fn changed(now: &BTreeMap<Key, Game>, then: &BTreeMap<Key, Game>) -> (u64, u64, u64) {
    let (mut gained, mut lost, mut both) = (0, 0, 0);
    for (key, game) in now {
        let Some(before) = then.get(key) else {
            continue;
        };
        match (game.result, before.result) {
            (Result::Open, _) | (_, Result::Open) => continue,
            (Result::Won, Result::Lost) => gained += 1,
            (Result::Lost, Result::Won) => lost += 1,
            _ => {}
        }
        both += 1;
    }
    (gained, lost, both)
}

/// The per-deck rule: how many decks the level sat at in ten seeds or more,
/// and those of them it turned against more often than for.
fn deck_rule(per_deck: &BTreeMap<String, Deck>) -> (usize, Vec<&String>) {
    let ten = |d: &Deck| d.with_level.decided + d.with_level.open >= 10;
    let decks = per_deck.values().filter(|d| ten(d)).count();
    let broken = per_deck
        .iter()
        .filter(|(_, d)| ten(d) && d.turned_against > d.turned_for)
        .map(|(deck, _)| deck)
        .collect();
    (decks, broken)
}

/// The per-deck rule's line, then each deck's line.
fn print_decks(level: &str, per_deck: &BTreeMap<String, Deck>, decks: usize, broken: &[&String]) {
    let names: Vec<&str> = broken.iter().map(|deck| deck.as_str()).collect();
    println!(
        "per-deck rule: {} of {decks} decks with ten seeds or more turned against more often than for{}",
        broken.len(),
        if names.is_empty() {
            String::new()
        } else {
            format!(" ({})", names.join(", "))
        }
    );
    println!("per deck: its side with {level}, and the seeds the level alone turned");
    for (deck, d) in per_deck {
        println!(
            "  {deck:48} {:3} of {:3} ({:5.1} %)  turned {:2} for, {:2} against",
            d.with_level.won,
            d.with_level.decided,
            100.0 * rate(d.with_level.won, d.with_level.decided),
            d.turned_for,
            d.turned_against
        );
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let runs: Vec<Value> = args.runs.iter().map(read).collect::<anyhow::Result<_>>()?;
    let level = match &args.level {
        Some(level) => level.clone(),
        None => runs[0]["baseline"].as_str().unwrap_or("?").to_owned(),
    };
    for key in [
        "seats",
        "teams",
        "first_seed",
        "decks_manifest",
        "deck_split",
    ] {
        ensure!(
            runs.iter().all(|run| run[key] == runs[0][key]),
            "the runs differ in {key}"
        );
    }
    let seats = usize::try_from(runs[0]["seats"].as_u64().unwrap_or(2)).unwrap_or(2);
    let teams: Vec<u64> = runs[0]["teams"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect();
    let (other, games, rows) = collect(&runs, &level, seats, &teams)?;
    let mut all = Share::default();
    for game in games.values() {
        all.add(game.result);
    }
    let (sides, per_deck, (turned_for, turned_against)) = tally(&games, seats);
    let p = sign_test(turned_for, turned_against);
    let (decks_of_ten, broken) = deck_rule(&per_deck);

    let (low, high) = wilson(all.won, all.decided);
    println!(
        "{level} against {other}: won {} of {} decided games ({:.1} %, Wilson 95 % {:.1}-{:.1}), {} open; {} games from {rows} rows",
        all.won,
        all.decided,
        100.0 * rate(all.won, all.decided),
        100.0 * low,
        100.0 * high,
        all.open,
        games.len()
    );
    println!(
        "seeds by sides won: {}",
        sides
            .iter()
            .map(|(label, n)| format!("{label}: {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "sign test over the seeds the level turned: {turned_for} for, {turned_against} against, two-sided p = {p:.4}"
    );
    print_decks(&level, &per_deck, decks_of_ten, &broken);
    let changed = if args.before.is_empty() {
        None
    } else {
        let before: Vec<Value> = args
            .before
            .iter()
            .map(read)
            .collect::<anyhow::Result<_>>()?;
        let (_, then, _) = collect(&before, &level, seats, &teams)?;
        let (gained, lost, both) = changed(&games, &then);
        println!(
            "against the build before, over {both} games both played: {gained} won that were lost, {lost} lost that were won, two-sided p = {:.4}",
            sign_test(gained, lost)
        );
        Some(json!({"games": both, "gained": gained, "lost": lost, "p": sign_test(gained, lost)}))
    };
    if let Some(out) = &args.out {
        let report = json!({
            "level": level,
            "against": other,
            "rows": rows,
            "games": all.json(),
            "seeds_by_sides_won": sides,
            "sign_test": {"for": turned_for, "against": turned_against, "p": p},
            "against_before": changed,
            "per_deck_rule": {"decks_with_ten_seeds": decks_of_ten, "turned_against": broken},
            "per_deck": per_deck.iter().map(|(deck, d)| (deck.clone(), json!({
                "with_level": d.with_level.json(),
                "with_other": d.with_other.json(),
                "turned_for": d.turned_for,
                "turned_against": d.turned_against,
            }))).collect::<BTreeMap<_, _>>(),
        });
        std::fs::write(out, serde_json::to_vec_pretty(&report)?)
            .with_context(|| format!("writing {}", out.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(focal_seat: u64, utility: f64) -> Value {
        json!({"seed": 1, "focal_seat": focal_seat, "base_utility": utility,
               "base_capped": false, "decks": ["a", "b", "c", "d"]})
    }

    /// A row's game is the seed and the level's seats: in two against two
    /// the focal seats 0 and 2 are one side, so their rows are one game.
    #[test]
    fn a_game_is_its_seed_and_the_levels_seats() {
        let teams = [1, 2, 1, 2];
        assert_eq!(focal_side(&row(0, 1.0), &teams, 4), [0, 2]);
        assert_eq!(focal_side(&row(2, 1.0), &teams, 4), [0, 2]);
        assert_eq!(focal_side(&row(1, 1.0), &teams, 4), [1, 3]);
        assert_eq!(
            focal_side(&row(1, 1.0), &[], 2),
            [1],
            "a duel's side is its seat"
        );
        assert_eq!(focal(&row(0, 1.0)), Result::Won);
        assert_eq!(flipped(focal(&row(0, 1.0))), Result::Lost);
        assert_eq!(focal(&row(0, 0.5)), Result::Open);
        let (low, high) = wilson(50, 100);
        assert!(low < 0.5 && high > 0.5 && (0.5 - low - (high - 0.5)).abs() < 1e-9);
    }

    /// The sign test against values worked out by hand: 9 of 10 is 22/1024
    /// two-sided, an even split is 1, nothing turned is 1, and a lopsided
    /// thousand neither underflows nor exceeds one.
    #[test]
    fn the_sign_test_is_the_two_sided_binomial_tail() {
        assert!((sign_test(9, 1) - 22.0 / 1024.0).abs() < 1e-12);
        assert!((sign_test(1, 9) - 22.0 / 1024.0).abs() < 1e-12);
        assert!((sign_test(5, 5) - 1.0).abs() < 1e-12);
        assert!((sign_test(0, 0) - 1.0).abs() < 1e-12);
        assert!((sign_test(3, 0) - 0.25).abs() < 1e-12);
        let big = sign_test(600, 400);
        assert!(big > 0.0 && big < 1e-9, "{big}");
        assert!((sign_test(500, 500) - 1.0).abs() < 1e-9);
        assert!(sign_test(133, 90) < 0.01 && sign_test(133, 90) > 0.001);
    }

    /// Against an earlier build the games are compared one by one: a game
    /// counts where both builds played it to a decision, and only those
    /// that went another way say which build is better.
    #[test]
    fn games_are_compared_with_an_earlier_build_one_by_one() {
        let games = |results: &[(u64, Result)]| -> BTreeMap<Key, Game> {
            results
                .iter()
                .map(|&(seed, result)| {
                    let game = Game {
                        decks: Vec::new(),
                        result,
                    };
                    ((seed, vec![0]), game)
                })
                .collect()
        };
        let then = games(&[
            (1, Result::Lost),
            (2, Result::Won),
            (3, Result::Won),
            (4, Result::Open),
            (5, Result::Lost),
        ]);
        let now = games(&[
            (1, Result::Won),
            (2, Result::Won),
            (3, Result::Lost),
            (4, Result::Won),
            (5, Result::Won),
            (6, Result::Won),
        ]);
        assert_eq!(changed(&now, &then), (2, 1, 4));
        assert_eq!(changed(&then, &now), (1, 2, 4));
    }
}
