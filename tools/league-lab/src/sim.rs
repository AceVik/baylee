use crate::league::{self, CHAMPION_LIMIT, POLICIES, Policy, Record, SEASON};
use rand::{RngExt, SeedableRng, seq::SliceRandom};
use rand_chacha::ChaCha8Rng;
use rand_distr::StandardNormal;

pub const DAYS: u16 = 4 * SEASON;
const WIDTH: i32 = 200;

#[derive(Clone, Copy)]
pub struct Scenario {
    pub label: &'static str,
    pub population: usize,
    pub spread: f64,
    pub seed: u64,
}

struct Pilot {
    skill: i32,
    deck: i32,
    daily: u8,
    join: u16,
    leave: u16,
}
fn normal(rng: &mut ChaCha8Rng, sigma: f64) -> i32 {
    let z: f64 = rng.sample(StandardNormal);
    (z * sigma).round() as i32
}
fn pilots(cfg: Scenario, rng: &mut ChaCha8Rng) -> Vec<Pilot> {
    (0..cfg.population)
        .map(|_| {
            let daily = match rng.random_range(0..100) {
                0..35 => 2,
                35..70 => 6,
                _ => 10,
            };
            let join = if rng.random_ratio(1, 8) {
                rng.random_range(14..60)
            } else {
                0
            };
            let leave = if rng.random_ratio(1, 10) {
                rng.random_range(240..330)
            } else {
                DAYS
            };
            Pilot {
                skill: 1500 + normal(rng, cfg.spread),
                deck: 0,
                daily,
                join,
                leave,
            }
        })
        .collect()
}

struct Odds(Vec<f64>);
impl Odds {
    fn new() -> Self {
        Self(
            (-4000..=4000)
                .map(|d| 1.0 / (1.0 + 10_f64.powf(-f64::from(d) / 400.0)))
                .collect(),
        )
    }
    fn at(&self, d: i32) -> f64 {
        self.0[(d.clamp(-4000, 4000) + 4000) as usize]
    }
}

// Shuffle within Elo bands. A random offset changes their boundaries each round.
// Carry an odd player into the next nonempty band; at most one bye per round.
fn play_day(
    day: u16,
    pilots: &[Pilot],
    records: &mut [Record],
    odds: &Odds,
    rng: &mut ChaCha8Rng,
) -> u64 {
    let mut bands: [Vec<usize>; 48] = std::array::from_fn(|_| Vec::new());
    let mut games = 0;
    for round in 0..10 {
        let width = if rng.random_ratio(1, 5) { 600 } else { WIDTH };
        let offset = rng.random_range(0..width);
        for (id, pilot) in pilots.iter().enumerate() {
            if pilot.join <= day && day < pilot.leave && round < pilot.daily {
                let bucket = ((records[id].elo + offset + 1600) / width).clamp(0, 47) as usize;
                bands[bucket].push(id);
            }
        }
        let mut waiting: Option<usize> = None;
        for band in &mut bands {
            band.shuffle(rng);
            for id in band.drain(..) {
                if let Some(other) = waiting.take() {
                    let a = &pilots[id];
                    let b: &Pilot = &pilots[other];
                    let win = odds.at(a.skill + a.deck - b.skill - b.deck);
                    let score = f64::from(u8::from(rng.random::<f64>() < win));
                    let expected = odds.at(records[id].elo - records[other].elo);
                    let delta = (24.0 * (score - expected)).round() as i32;
                    records[id].played(day, delta);
                    records[other].played(day, -delta);
                    games += 1;
                } else {
                    waiting = Some(id);
                }
            }
        }
    }
    games
}

struct History {
    first_master: Vec<Option<u16>>,
    ever_gm: Vec<bool>,
    last_week: Vec<usize>,
}
impl History {
    fn new(n: usize) -> Self {
        Self {
            first_master: vec![None; n],
            ever_gm: vec![false; n],
            last_week: vec![0; n],
        }
    }
    fn update(&mut self, tiers: &[usize], records: &[Record], day: u16) {
        for (id, &tier) in tiers.iter().enumerate() {
            if tier >= 7 {
                let elapsed = day - records[id].first;
                assert!(elapsed >= league::MASTER_AGE && records[id].games >= league::MASTER_GAMES);
                self.first_master[id].get_or_insert(elapsed);
            }
            self.ever_gm[id] |= tier >= 8;
            if day == DAYS - 8 {
                self.last_week[id] = tier;
            }
        }
    }
}

pub fn header() -> String {
    format!(
        "scenario,seed,population,day,policy,matches,{},champ_candidates,ever_master,ever_gm,ten_daily,ten_master,ten_first_min,ten_first_p50,ten_first_p90,champ_last_week,champ_retained,master_true_top1,master_last_week,master_retained,gm_last_week,gm_retained",
        league::NAMES.join(",")
    )
}

fn percentile(sorted: &[u16], p: usize) -> String {
    if sorted.is_empty() {
        return String::new();
    }
    sorted[(sorted.len() - 1) * p / 100].to_string()
}

#[allow(clippy::too_many_arguments)]
fn row(
    cfg: Scenario,
    policy: Policy,
    day: u16,
    matches: u64,
    table: &league::Standings,
    history: &History,
    pilots: &[Pilot],
    top_one: i32,
) -> String {
    let mut counts = [0; 10];
    let mut first = Vec::new();
    let mut ten = 0;
    let mut retained = 0;
    let mut strong = 0;
    for (id, &tier) in table.tiers.iter().enumerate() {
        counts[tier] += 1;
        if pilots[id].daily == 10 {
            ten += 1;
            if let Some(age) = history.first_master[id] {
                first.push(age);
            }
        }
        retained += usize::from(tier == 9 && history.last_week[id] == 9);
        strong += usize::from(tier >= 7 && pilots[id].skill >= top_one);
    }
    assert!(counts[9] <= CHAMPION_LIMIT);
    first.sort_unstable();
    let counts = counts.map(|n| n.to_string()).join(",");
    let ever_master = history.first_master.iter().flatten().count();
    let ever_gm = history.ever_gm.iter().filter(|&&v| v).count();
    let retention = |level| {
        (
            history
                .last_week
                .iter()
                .filter(|&&tier| tier >= level)
                .count(),
            table
                .tiers
                .iter()
                .zip(&history.last_week)
                .filter(|&(a, b)| *a >= level && *b >= level)
                .count(),
        )
    };
    let (last_week, _) = retention(9);
    let (last_master, held_master) = retention(7);
    let (last_gm, held_gm) = retention(8);
    format!(
        "{},{},{},{},{},{matches},{counts},{},{ever_master},{ever_gm},{ten},{},{},{},{},{last_week},{retained},{strong},{last_master},{held_master},{last_gm},{held_gm}",
        cfg.label,
        cfg.seed,
        cfg.population,
        day + 1,
        policy.name,
        table.candidates,
        first.len(),
        percentile(&first, 0),
        percentile(&first, 50),
        percentile(&first, 90)
    )
}

pub fn simulate(cfg: Scenario) -> Vec<String> {
    let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed);
    let mut players = pilots(cfg, &mut rng);
    let mut skill: Vec<_> = players.iter().map(|p| p.skill).collect();
    let idx = skill.len() * 99 / 100;
    let top_one = *skill.select_nth_unstable(idx).1;
    let mut records = vec![Record::default(); players.len()];
    let mut history = POLICIES.map(|_| History::new(players.len()));
    let odds = Odds::new();
    let mut matches = 0;
    let mut rows = Vec::new();
    for day in 0..DAYS {
        if day % SEASON == 0 {
            for p in &mut players {
                p.deck = normal(&mut rng, 60.0);
            }
        }
        matches += play_day(day, &players, &mut records, &odds, &mut rng);
        for (policy, hist) in POLICIES.into_iter().zip(&mut history) {
            let table = league::standings(&records, policy, day);
            hist.update(&table.tiers, &records, day);
            if (day + 1) % SEASON == 0 {
                rows.push(row(
                    cfg, policy, day, matches, &table, hist, &players, top_one,
                ));
            }
        }
    }
    rows
}
