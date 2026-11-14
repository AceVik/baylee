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
pub(crate) struct Tally {
    pub(crate) games: u64,
    pub(crate) wins: u64,
    pub(crate) draws: u64,
    pub(crate) fell_back: u64,
    pub(crate) answers: u64,
    pub(crate) think_ms: u64,
    pub(crate) tokens: u64,
}

impl Tally {
    pub(crate) fn add(&mut self, other: &Self) {
        self.games += other.games;
        self.wins += other.wins;
        self.draws += other.draws;
        self.fell_back += other.fell_back;
        self.answers += other.answers;
        self.think_ms += other.think_ms;
        self.tokens += other.tokens;
    }
}

pub(crate) type Pairs = BTreeMap<(String, String), Tally>;

#[derive(Clone, Debug)]
struct LegOutcome {
    profiles: [String; 2],
    winner_seat: Option<u64>,
    draw: bool,
}

#[derive(Default, Clone, Debug)]
struct DuplicateStats {
    deals: u64,
    sweeps_won: u64,
    splits: u64,
    sweeps_lost: u64,
    half_points: u64,
    seat0_games: u64,
    seat0_wins: u64,
    seat1_games: u64,
    seat1_wins: u64,
}

#[derive(Default)]
struct DuplicateSummary {
    complete_deals: u64,
    decisive_sweeps: u64,
    same_seat_splits: u64,
    drawn_deals: u64,
    by_profile: BTreeMap<String, DuplicateStats>,
}

/// The report over every admitted game under `roots`.
pub fn report(roots: &[PathBuf]) -> anyhow::Result<String> {
    let mut pairs = Pairs::new();
    let mut deals: BTreeMap<(String, String), [Option<LegOutcome>; 2]> = BTreeMap::new();
    for root in roots {
        let attempts = root.join("attempts");
        anyhow::ensure!(attempts.is_dir(), "{} holds no attempts", root.display());
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&attempts)?
            .flatten()
            .map(|entry| entry.path())
            .collect();
        dirs.sort();
        let root_key = root.display().to_string();
        for dir in dirs {
            let Ok(raw) = std::fs::read(dir.join("receipt.json")) else {
                continue;
            };
            let Ok(receipt) = serde_json::from_slice::<Value>(&raw) else {
                continue;
            };
            if receipt["status"] == "admitted" {
                add_game(&dir, &receipt, &mut pairs);
                record_duplicate_leg(&root_key, &receipt, &mut deals);
            }
        }
    }
    let dup = summarize_duplicates(&deals);
    Ok(render(&pairs, &dup))
}

pub(crate) fn add_game(dir: &Path, receipt: &Value, pairs: &mut Pairs) {
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

fn record_duplicate_leg(
    root_key: &str,
    receipt: &Value,
    deals: &mut BTreeMap<(String, String), [Option<LegOutcome>; 2]>,
) {
    let Some(deal_id) = receipt["duplicate"]["deal_id"].as_str() else {
        return;
    };
    let Some(leg) = receipt["duplicate"]["leg"]
        .as_u64()
        .filter(|&l| l < 2)
        .and_then(|l| usize::try_from(l).ok())
    else {
        return;
    };
    let request = &receipt["request"];
    let name = |seat: usize| {
        request["profiles"][seat]
            .as_str()
            .or_else(|| request["models"][seat].as_str())
            .unwrap_or("unknown")
            .to_owned()
    };
    let terminal = &receipt["technical"]["terminal"];
    let entry = deals
        .entry((root_key.to_owned(), deal_id.to_owned()))
        .or_default();
    entry[leg] = Some(LegOutcome {
        profiles: [name(0), name(1)],
        winner_seat: terminal["winner_seat"].as_u64(),
        draw: terminal["draw"].as_bool().unwrap_or(false),
    });
}

fn summarize_duplicates(
    deals: &BTreeMap<(String, String), [Option<LegOutcome>; 2]>,
) -> DuplicateSummary {
    let mut summary = DuplicateSummary::default();
    for [leg0, leg1] in deals.values() {
        let (Some(l0), Some(l1)) = (leg0, leg1) else {
            continue;
        };
        if l0.profiles[0] != l1.profiles[1] || l0.profiles[1] != l1.profiles[0] {
            continue;
        }
        summary.complete_deals += 1;
        let pa = &l0.profiles[0];
        let pb = &l0.profiles[1];
        let pts_leg0_a = leg_half_points(l0, 0);
        let pts_leg1_a = leg_half_points(l1, 1);
        let total_a = pts_leg0_a + pts_leg1_a;
        let total_b = 4 - total_a;

        if total_a == 4 || total_b == 4 {
            summary.decisive_sweeps += 1;
        } else if !l0.draw && !l1.draw && l0.winner_seat == l1.winner_seat {
            summary.same_seat_splits += 1;
        } else {
            summary.drawn_deals += 1;
        }

        record_profile_deal(
            summary.by_profile.entry(pa.clone()).or_default(),
            total_a,
            !l0.draw && l0.winner_seat == Some(0),
            !l1.draw && l1.winner_seat == Some(1),
        );
        record_profile_deal(
            summary.by_profile.entry(pb.clone()).or_default(),
            total_b,
            !l1.draw && l1.winner_seat == Some(0),
            !l0.draw && l0.winner_seat == Some(1),
        );
    }
    summary
}

fn leg_half_points(leg: &LegOutcome, seat: u64) -> u64 {
    if leg.draw {
        1
    } else if leg.winner_seat == Some(seat) {
        2
    } else {
        0
    }
}

fn record_profile_deal(st: &mut DuplicateStats, half_pts: u64, won_seat0: bool, won_seat1: bool) {
    st.deals += 1;
    st.half_points += half_pts;
    st.seat0_games += 1;
    st.seat1_games += 1;
    st.seat0_wins += u64::from(won_seat0);
    st.seat1_wins += u64::from(won_seat1);
    match half_pts {
        3 | 4 => st.sweeps_won += 1,
        2 => st.splits += 1,
        _ => st.sweeps_lost += 1,
    }
}

fn render_duplicates(dup: &DuplicateSummary, ladder: &[LadderRow<'_>]) -> String {
    if dup.complete_deals == 0 {
        return String::new();
    }
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\nduplicate deals (seed- & deck-mirrored pairs, {} complete 2-leg deals):",
        dup.complete_deals
    );
    let _ = writeln!(
        out,
        "  variance cancelled: {}/{} splits 1-1 (same deck+seat won both legs: {}, drawn: {}), {}/{} won 2-0/1.5-0.5 by pilot skill",
        dup.same_seat_splits + dup.drawn_deals,
        dup.complete_deals,
        dup.same_seat_splits,
        dup.drawn_deals,
        dup.decisive_sweeps,
        dup.complete_deals,
    );
    let _ = writeln!(
        out,
        "{:<18} {:>5}  {:>5} {:>5} {:>5}  {:<16} {:>9} {:>9}",
        "profile", "deals", "won", "split", "lost", "deal score", "seat0 win", "seat1 win"
    );
    let elo_order: BTreeMap<&str, usize> = ladder
        .iter()
        .enumerate()
        .map(|(idx, row)| (row.profile, idx))
        .collect();
    let mut rows: Vec<(&str, &DuplicateStats)> = dup
        .by_profile
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    rows.sort_by(|(pa, sa), (pb, sb)| {
        let pct_a = if sa.deals == 0 {
            0.0
        } else {
            real(sa.half_points) / real(sa.deals * 4)
        };
        let pct_b = if sb.deals == 0 {
            0.0
        } else {
            real(sb.half_points) / real(sb.deals * 4)
        };
        pct_b
            .total_cmp(&pct_a)
            .then_with(|| elo_order.get(pa).cmp(&elo_order.get(pb)))
            .then_with(|| pa.cmp(pb))
    });
    for (prof, st) in rows {
        let pts = 0.5 * real(st.half_points);
        let max_pts = real(st.deals * 2);
        let pct = if st.deals == 0 {
            0.0
        } else {
            100.0 * pts / max_pts
        };
        let s0_pct = if st.seat0_games == 0 {
            0.0
        } else {
            100.0 * real(st.seat0_wins) / real(st.seat0_games)
        };
        let s1_pct = if st.seat1_games == 0 {
            0.0
        } else {
            100.0 * real(st.seat1_wins) / real(st.seat1_games)
        };
        let _ = writeln!(
            out,
            "{prof:<18} {:>5}  {:>5} {:>5} {:>5}  {:>5.1}/{:<5.1} {:>3.0}% {:>8.0}% {:>8.0}%",
            st.deals, st.sweeps_won, st.splits, st.sweeps_lost, pts, max_pts, pct, s0_pct, s1_pct,
        );
    }
    out
}

fn render(pairs: &Pairs, dup: &DuplicateSummary) -> String {
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
    let ladder = bradley_terry_ladder(pairs, &totals);
    if ladder.len() >= 2 {
        let _ = writeln!(
            out,
            "\nhierarchy (Bradley-Terry Elo, anchor 1500):\n{:>4}  {:<18} {:>6} {:>6}  {:<26} {:>5} {:>8} {:>8}",
            "rank",
            "profile",
            "elo",
            "±95%",
            "pvp: won/games  win [95 %]",
            "drew",
            "ms/ans",
            "tok/ans"
        );
        for (idx, row) in ladder.iter().enumerate() {
            let _ = writeln!(
                out,
                "{:>4}  {:<18} {:>6.0} {:>6}  {:<26} {:>5} {:>8} {:>8}",
                idx + 1,
                row.profile,
                row.elo,
                format!("±{:.0}", row.ci95),
                share(&row.pvp),
                row.pvp.draws,
                per(row.all.think_ms, row.all.answers),
                per(row.all.tokens, row.all.answers),
            );
        }
    }
    out.push_str(&render_duplicates(dup, &ladder));
    out
}

pub(crate) struct LadderRow<'a> {
    pub(crate) profile: &'a str,
    pub(crate) elo: f64,
    pub(crate) ci95: f64,
    pub(crate) pvp: Tally,
    pub(crate) all: Tally,
}

/// Computes regularized Bradley-Terry MLE Elo ratings (anchored at mean 1500)
/// across all non-house profiles that played at least one head-to-head game.
pub(crate) fn bradley_terry_ladder<'a>(
    pairs: &'a Pairs,
    totals: &BTreeMap<&'a str, (Tally, Tally)>,
) -> Vec<LadderRow<'a>> {
    let mut profiles: Vec<&'a str> = Vec::new();
    let mut pvp_totals: BTreeMap<&'a str, Tally> = BTreeMap::new();
    for ((me, them), tally) in pairs {
        if house_level(me).is_some() || house_level(them).is_some() || me == them {
            continue;
        }
        if tally.games > 0 {
            pvp_totals.entry(me.as_str()).or_default().add(tally);
        }
    }
    for &p in pvp_totals.keys() {
        profiles.push(p);
    }
    let k = profiles.len();
    if k < 2 {
        return Vec::new();
    }
    // Weak symmetric Beta(0.5, 0.5) prior (1 virtual game against the 1500 anchor)
    // keeps undefeated or winless profiles finite while preserving exact ordering.
    let prior_wins = 0.5_f64;
    let prior_games = 2.0 * prior_wins;
    let mut gamma = vec![1.0_f64; k];
    let mut next = vec![1.0_f64; k];
    for _ in 0..400 {
        for i in 0..k {
            let me = profiles[i];
            let mut score = prior_wins;
            let mut denom = prior_games / (gamma[i] + 1.0);
            for (j, &them) in profiles.iter().enumerate() {
                if i == j {
                    continue;
                }
                if let Some(t) = pairs.get(&(me.to_owned(), them.to_owned()))
                    && t.games > 0
                {
                    score += real(t.wins) + 0.5 * real(t.draws);
                    denom += real(t.games) / (gamma[i] + gamma[j]);
                }
            }
            next[i] = (score / denom).clamp(1e-6, 1e6);
        }
        let mean_ln =
            next.iter().map(|g| g.ln()).sum::<f64>() / f64::from(u32::try_from(k).unwrap_or(1));
        let scale = (-mean_ln).exp();
        for (g, &n) in gamma.iter_mut().zip(&next) {
            *g = n * scale;
        }
    }
    let elo_scale = 400.0 / 10.0_f64.ln();
    let mut rows: Vec<LadderRow<'a>> = (0..k)
        .map(|i| {
            let me = profiles[i];
            let gi = gamma[i];
            let mut info = prior_games * gi / (gi + 1.0).powi(2);
            for (j, &them) in profiles.iter().enumerate() {
                if i == j {
                    continue;
                }
                if let Some(t) = pairs.get(&(me.to_owned(), them.to_owned()))
                    && t.games > 0
                {
                    let gj = gamma[j];
                    info += real(t.games) * (gi * gj) / (gi + gj).powi(2);
                }
            }
            let elo = 1500.0 + elo_scale * gi.ln();
            let ci95 = 1.959_964 * elo_scale / info.max(1e-9).sqrt();
            let pvp = pvp_totals.get(me).cloned().unwrap_or_default();
            let all = totals
                .get(me)
                .map_or_else(Default::default, |(a, _)| a.clone());
            LadderRow {
                profile: me,
                elo,
                ci95,
                pvp,
                all,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.elo
            .total_cmp(&a.elo)
            .then_with(|| a.profile.cmp(b.profile))
    });
    rows
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
            text.contains("hierarchy (Bradley-Terry Elo, anchor 1500):"),
            "{text}"
        );
        let sol_pos = text.find("1  sol").expect("sol ranked #1");
        let astra_pos = text.find("2  astra").expect("astra ranked #2");
        assert!(sol_pos < astra_pos, "{text}");
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

    #[test]
    fn duplicate_summary_distinguishes_2_0_sweeps_from_1_1_deck_splits() {
        let mut deals: BTreeMap<(String, String), [Option<LegOutcome>; 2]> = BTreeMap::new();
        // Deal 0: sol beats astra in both legs (Seat 0 in leg 0, Seat 1 in leg 1) -> 2-0 sweep.
        deals.insert(
            ("r".into(), "deal-0".into()),
            [
                Some(LegOutcome {
                    profiles: ["sol".into(), "astra".into()],
                    winner_seat: Some(0),
                    draw: false,
                }),
                Some(LegOutcome {
                    profiles: ["astra".into(), "sol".into()],
                    winner_seat: Some(1),
                    draw: false,
                }),
            ],
        );
        // Deal 1: Seat 0 wins both legs (sol wins leg 0, astra wins leg 1) -> 1-1 split.
        deals.insert(
            ("r".into(), "deal-1".into()),
            [
                Some(LegOutcome {
                    profiles: ["sol".into(), "astra".into()],
                    winner_seat: Some(0),
                    draw: false,
                }),
                Some(LegOutcome {
                    profiles: ["astra".into(), "sol".into()],
                    winner_seat: Some(0),
                    draw: false,
                }),
            ],
        );
        let dup = summarize_duplicates(&deals);
        assert_eq!(dup.complete_deals, 2);
        assert_eq!(dup.decisive_sweeps, 1);
        assert_eq!(dup.same_seat_splits, 1);
        let sol = &dup.by_profile["sol"];
        assert_eq!(
            (sol.deals, sol.sweeps_won, sol.splits, sol.sweeps_lost),
            (2, 1, 1, 0)
        );
        assert_eq!(sol.half_points, 6);
    }
}
