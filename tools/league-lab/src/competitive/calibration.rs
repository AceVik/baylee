use super::rating::{Encounter, Odds, Player, Rule, settle};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Copy)]
struct Fixture {
    name: &'static str,
    start: i32,
    strength: i32,
    opponent: i32,
    games: u16,
    opponent_games: u16,
    luck: f64,
}
const FIXTURES: [Fixture; 6] = [
    Fixture {
        name: "calibrated-favourite",
        start: 1950,
        strength: 1950,
        opponent: 1500,
        games: 100,
        opponent_games: 100,
        luck: 0.04,
    },
    Fixture {
        name: "overrated-veteran",
        start: 1950,
        strength: 1450,
        opponent: 1500,
        games: 100,
        opponent_games: 100,
        luck: 0.04,
    },
    Fixture {
        name: "underrated-newcomer",
        start: 1450,
        strength: 1950,
        opponent: 1900,
        games: 0,
        opponent_games: 100,
        luck: 0.04,
    },
    Fixture {
        name: "surprising-placement-opponents",
        start: 1950,
        strength: 1950,
        opponent: 1500,
        games: 100,
        opponent_games: 0,
        luck: 0.04,
    },
    Fixture {
        name: "low-variance",
        start: 1950,
        strength: 1950,
        opponent: 1500,
        games: 100,
        opponent_games: 100,
        luck: 0.0,
    },
    Fixture {
        name: "high-variance",
        start: 1950,
        strength: 1950,
        opponent: 1500,
        games: 100,
        opponent_games: 100,
        luck: 0.10,
    },
];
const TRIALS: u32 = 12_000;
#[derive(Default)]
struct Stats {
    queue10: f64,
    errors: [f64; 3],
    surprises: u64,
    escalated: u64,
    penalty: f64,
    max: f64,
    in_range: Vec<u16>,
}
fn run_fixture(f: Fixture, seed: u64, odds: &Odds) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let rules = [Rule::Classic, Rule::FixedCap, Rule::Evidence];
    let mut stats = rules.map(|_| Stats::default());
    for _ in 0..TRIALS {
        let mut focal = rules.map(|_| Player::at(f.start, f.games));
        let mut reached = [None; 3];
        for game in 1..=100_u16 {
            let opponent = f.opponent + rng.random_range(-60..=60);
            // Placement opponents can have much more skill than their public score.
            let true_opponent = opponent + if f.opponent_games == 0 { 300 } else { 0 };
            let win = rng.random::<f64>() < odds.at(f.strength - true_opponent, f.luck);
            for (i, rule) in rules.iter().copied().enumerate() {
                let p = &mut focal[i];
                let mut rival = Player::at(opponent, f.opponent_games);
                let update = settle(
                    [p, &mut rival],
                    Encounter {
                        ids: [0, usize::from(game)],
                        now: u32::from(game) * 3600,
                        repeats: 0,
                        a_won: win,
                    },
                    rule,
                    odds,
                );
                let s = &mut stats[i];
                let err = f64::from((p.record.elo - f.strength).abs());
                if err <= 100.0 {
                    reached[i].get_or_insert(game);
                }
                if game == 10 {
                    s.queue10 += f64::from((p.queue_elo(rule) - f.strength).abs());
                }
                if let Some(idx) = [20, 50, 100].iter().position(|&n| n == game) {
                    s.errors[idx] += err;
                }
                if update.wide && update.upset {
                    s.surprises += 1;
                    let is_low = update.low == 0;
                    let relevant = if is_low {
                        update.streak[0]
                    } else {
                        update.streak[1]
                    };
                    s.escalated += u64::from(relevant > 0 && (is_low || f.opponent_games >= 20));
                    s.penalty += update.delta[0].abs();
                    s.max = s.max.max(update.delta[0].abs());
                }
            }
        }
        for (s, first) in stats.iter_mut().zip(reached) {
            if let Some(n) = first {
                s.in_range.push(n);
            }
        }
    }
    for (rule, mut s) in rules.into_iter().zip(stats) {
        s.in_range.sort_unstable();
        let n = f64::from(TRIALS);
        let errors = s.errors.map(|v| format!("{:.3}", v / n)).join(",");
        println!(
            "{},{},{seed},{TRIALS},{},{:.3},{errors},{},{:.4},{:.4},{:.4},{},{}",
            f.name,
            rule.name(),
            100 * TRIALS,
            s.queue10 / n,
            s.surprises,
            100.0 * s.escalated as f64 / s.surprises.max(1) as f64,
            s.penalty / s.surprises.max(1) as f64,
            s.max,
            s.in_range.len(),
            s.in_range
                .get(s.in_range.len().saturating_sub(1) / 2)
                .map_or(String::new(), u16::to_string)
        );
    }
}
pub(super) fn run() {
    println!(
        "fixture,rule,seed,episodes,matches,queue_mae10,public_mae20,public_mae50,public_mae100,wide_upsets,evidence_pct,mean_focal_upset_delta_abs,max_focal_upset_delta_abs,reached_100,first_within_100_p50"
    );
    let odds = Odds::new();
    for (i, f) in FIXTURES.into_iter().enumerate() {
        run_fixture(f, 80100 + i as u64, &odds);
    }
}
