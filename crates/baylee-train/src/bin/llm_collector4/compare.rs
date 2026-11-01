//! `compare`: the admitted games of collector roots, profile against
//! profile. A seat is named by the profile its pairing asked for (a
//! `house:<level>` by that); a fallback that took over from it is counted
//! beside its results, never moved to the fallback's name, so a profile
//! that mostly fell back shows as such and not as its own strength. The
//! win share is wins over games (a draw is not won), with a Wilson 95 %
//! interval; think time and tokens are those of the mind's own answers
//! (the seat transcript's `answered` rows `by: mind`, fallbacks' included).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::schedule::house_level;

/// One profile's games against one opponent.
#[derive(Default, Clone)]
struct Tally {
    games: u64,
    wins: u64,
    draws: u64,
    fell_back: u64,
    answers: u64,
    think_ms: u64,
    tokens: u64,
}

impl Tally {
    fn add(&mut self, other: &Self) {
        self.games += other.games;
        self.wins += other.wins;
        self.draws += other.draws;
        self.fell_back += other.fell_back;
        self.answers += other.answers;
        self.think_ms += other.think_ms;
        self.tokens += other.tokens;
    }
}

type Pairs = BTreeMap<(String, String), Tally>;

/// The report over every admitted game under `roots`.
pub fn report(roots: &[PathBuf]) -> anyhow::Result<String> {
    let mut pairs = Pairs::new();
    for root in roots {
        let attempts = root.join("attempts");
        anyhow::ensure!(attempts.is_dir(), "{} holds no attempts", root.display());
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&attempts)?
            .flatten()
            .map(|entry| entry.path())
            .collect();
        dirs.sort();
        for dir in dirs {
            let Ok(raw) = std::fs::read(dir.join("receipt.json")) else {
                continue;
            };
            let Ok(receipt) = serde_json::from_slice::<Value>(&raw) else {
                continue;
            };
            if receipt["status"] == "admitted" {
                add_game(&dir, &receipt, &mut pairs);
            }
        }
    }
    Ok(render(&pairs))
}

fn add_game(dir: &Path, receipt: &Value, pairs: &mut Pairs) {
    let request = &receipt["request"];
    let technical = &receipt["technical"];
    let name = |seat: usize| {
        request["profiles"][seat]
            .as_str()
            .or_else(|| request["models"][seat].as_str())
            .unwrap_or("unknown")
            .to_owned()
    };
    let draw = technical["terminal"]["draw"].as_bool().unwrap_or(false);
    let winner = technical["terminal"]["winner_seat"].as_u64();
    for (seat, number) in [(0_usize, 0_u64), (1, 1)] {
        let me = name(seat);
        let house = house_level(&me).is_some();
        let facts = &technical["seats"][seat];
        let mut tally = Tally {
            games: 1,
            wins: u64::from(!draw && winner == Some(number)),
            draws: u64::from(draw),
            ..Tally::default()
        };
        if !house {
            let took_over = facts["profile_fallbacks"]["took_over"]
                .as_array()
                .is_some_and(|taken| !taken.is_empty());
            let house_answers = facts["transcript_answer_counts"]["house"]
                .as_u64()
                .unwrap_or(0);
            tally.fell_back = u64::from(took_over || house_answers > 0);
            mind_answers(dir, seat, &mut tally);
        }
        pairs.entry((me, name(1 - seat))).or_default().add(&tally);
    }
}

/// The seat's own answers: count, think time and tokens.
fn mind_answers(dir: &Path, seat: usize, tally: &mut Tally) {
    let suffix = format!("-seat{seat}.jsonl");
    let Ok(entries) = std::fs::read_dir(dir.join("transcripts")) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(&suffix))
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            let Ok(row) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if row["event"] != "answered" || row["by"] != "mind" {
                continue;
            }
            tally.answers += 1;
            tally.think_ms += row["model_ms"].as_u64().unwrap_or(0);
            let note = row["note"]
                .as_str()
                .and_then(|note| serde_json::from_str::<Value>(note).ok());
            if let Some(tokens) = note.as_ref().and_then(|note| note["tokens"].as_object()) {
                tally.tokens += tokens.values().filter_map(Value::as_u64).sum::<u64>();
            }
        }
    }
}

fn real(n: u64) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// The Wilson 95 % interval of `wins` in `games`.
fn wilson(wins: u64, games: u64) -> (f64, f64) {
    if games == 0 {
        return (0.0, 1.0);
    }
    let n = real(games);
    let p = real(wins) / n;
    let z2 = 1.959_964_f64.powi(2);
    let centre = (p + z2 / (2.0 * n)) / (1.0 + z2 / n);
    let half = (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() * 1.959_964 / (1.0 + z2 / n);
    ((centre - half).max(0.0), (centre + half).min(1.0))
}

/// `won/games  pp%  [lo–hi]`.
fn share(tally: &Tally) -> String {
    if tally.games == 0 {
        return "-".into();
    }
    let (lo, hi) = wilson(tally.wins, tally.games);
    format!(
        "{:>3}/{:<3} {:>3.0}% [{:>2.0}–{:>3.0}]",
        tally.wins,
        tally.games,
        100.0 * real(tally.wins) / real(tally.games),
        100.0 * lo,
        100.0 * hi
    )
}

fn per(total: u64, count: u64) -> String {
    if count == 0 {
        "-".into()
    } else {
        format!("{:.0}", real(total) / real(count))
    }
}

fn render(pairs: &Pairs) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<18} {:<18} {:<26} {:>5} {:>9}",
        "profile", "opponent", "won/games  win [95 %]", "drew", "fell back"
    );
    let mut totals: BTreeMap<&str, (Tally, Tally)> = BTreeMap::new();
    for ((me, them), tally) in pairs {
        if house_level(me).is_some() {
            continue;
        }
        let _ = writeln!(
            out,
            "{me:<18} {them:<18} {:<26} {:>5} {:>9}",
            share(tally),
            tally.draws,
            tally.fell_back
        );
        let (all, house) = totals.entry(me).or_default();
        all.add(tally);
        if house_level(them).is_some() {
            house.add(tally);
        }
    }
    let _ = writeln!(
        out,
        "\n{:<18} {:<26} {:<26} {:>7} {:>8} {:>8} {:>9}",
        "profile",
        "all: won/games  win [95 %]",
        "vs house",
        "ans/gm",
        "ms/ans",
        "tok/ans",
        "fell back"
    );
    for (me, (all, house)) in &totals {
        let _ = writeln!(
            out,
            "{me:<18} {:<26} {:<26} {:>7} {:>8} {:>8} {:>9}",
            share(all),
            share(house),
            per(all.answers, all.games),
            per(all.think_ms, all.answers),
            per(all.tokens, all.answers),
            all.fell_back
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::tests::TestDir;
    use serde_json::json;

    fn game(root: &Path, id: &str, profiles: [&str; 2], winner: u64, took_over: bool) {
        let dir = root.join("attempts").join(id);
        std::fs::create_dir_all(dir.join("transcripts")).unwrap();
        let fallbacks = |seat: usize| {
            if took_over && seat == 0 {
                json!({"took_over": [{"cause": "limited", "profile": "x", "turn": 1}]})
            } else {
                json!({"took_over": []})
            }
        };
        let receipt = json!({
            "status": "admitted",
            "request": {"profiles": profiles, "models": profiles},
            "technical": {
                "terminal": {"draw": false, "winner_seat": winner},
                "seats": [
                    {"profile_fallbacks": fallbacks(0), "transcript_answer_counts": {"house": 0}},
                    {"profile_fallbacks": fallbacks(1), "transcript_answer_counts": {"house": 0}},
                ],
            },
        });
        std::fs::write(dir.join("receipt.json"), receipt.to_string()).unwrap();
        let answered = json!({
            "event": "answered", "by": "mind", "model_ms": 1000,
            "note": json!({"tokens": {"input": 10, "output": 5, "cache_read": 85}}).to_string(),
        });
        let standing = json!({"event": "standing", "action": "PassPriority"});
        std::fs::write(
            dir.join("transcripts/g-seat0.jsonl"),
            format!("{answered}\n{standing}\n{answered}\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("transcripts/g-seat0-mind.jsonl"),
            format!("{answered}\n"),
        )
        .unwrap();
    }

    #[test]
    fn compare_counts_wins_takeovers_and_the_minds_answers_by_requested_profile() {
        let temp = TestDir::new("compare");
        let root = temp.path().join("run1");
        game(&root, "attempt-000000000", ["sol", "astra"], 0, false);
        game(&root, "attempt-000000001", ["sol", "house:sharp"], 1, true);
        game(&root, "attempt-000000002", ["astra", "sol"], 1, false);
        std::fs::create_dir_all(root.join("attempts/attempt-000000003")).unwrap();
        let mut pairs = Pairs::new();
        for id in [
            "attempt-000000000",
            "attempt-000000001",
            "attempt-000000002",
        ] {
            let dir = root.join("attempts").join(id);
            let receipt: Value =
                serde_json::from_slice(&std::fs::read(dir.join("receipt.json")).unwrap()).unwrap();
            add_game(&dir, &receipt, &mut pairs);
        }
        let sol_astra = &pairs[&("sol".to_owned(), "astra".to_owned())];
        assert_eq!((sol_astra.games, sol_astra.wins), (2, 2));
        let sol_house = &pairs[&("sol".to_owned(), "house:sharp".to_owned())];
        assert_eq!(
            (sol_house.games, sol_house.wins, sol_house.fell_back),
            (1, 0, 1)
        );
        let house_sol = &pairs[&("house:sharp".to_owned(), "sol".to_owned())];
        assert_eq!((house_sol.wins, house_sol.answers), (1, 0));
        // Seat 0's transcript only; its mind log and standing rows are not answers.
        let astra_sol = &pairs[&("astra".to_owned(), "sol".to_owned())];
        assert_eq!(
            (astra_sol.answers, astra_sol.think_ms, astra_sol.tokens),
            (2, 2000, 200)
        );
        assert_eq!(sol_astra.answers, 2);
        let text = report(&[root]).unwrap();
        assert!(text.contains("sol"), "{text}");
        assert!(
            !text.lines().any(|line| line.starts_with("house:")),
            "{text}"
        );
    }

    #[test]
    fn wilson_is_bounded_and_centred_on_the_share() {
        let (lo, hi) = wilson(35, 44);
        assert!(
            (0.65..0.70).contains(&lo) && (0.88..0.91).contains(&hi),
            "{lo} {hi}"
        );
        assert_eq!(wilson(0, 0), (0.0, 1.0));
        let (lo, hi) = wilson(0, 10);
        assert!(lo.abs() < 1e-12 && hi < 0.31);
    }
}
