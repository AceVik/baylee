use super::fixtures::{CASES, Case, DAY, Pilot, arrivals, normal, pilots, uniform};
use super::metrics::{HEADER, Metrics};
use super::queue::{Matcher, Pairs, Queue, Ticket};
use super::rating::{Encounter, Odds, Player, Rule, settle};
use crate::league::SEASON;
use crate::sim::DAYS;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
const POLICIES: [(Matcher, Rule); 8] = [
    (Matcher::Balanced, Rule::UnboundedLedger),
    (Matcher::Fixed, Rule::Classic),
    (Matcher::Fixed, Rule::Evidence),
    (Matcher::Balanced, Rule::Classic),
    (Matcher::Balanced, Rule::FixedCap),
    (Matcher::Balanced, Rule::Evidence),
    (Matcher::Adaptive, Rule::Evidence),
    (Matcher::Fifo, Rule::Evidence),
];
struct Round<'a> {
    tickets: &'a [Ticket],
    pilots: &'a [Pilot],
    elite: i32,
    odds: &'a Odds,
    seed: u64,
    slot: u32,
}
struct World {
    matcher: Matcher,
    rule: Rule,
    players: Vec<Player>,
    pairs: Pairs,
    stats: Metrics,
    divisions: crate::divisions::Comparison,
    decks: Option<super::deck_cycle::Field>,
    trace: Option<Vec<Played>>,
}
#[derive(Debug, PartialEq, Eq)]
struct Played {
    ids: [usize; 2],
    now: u32,
    won: bool,
    elo: [i32; 2],
}
impl World {
    fn new(n: usize, matcher: Matcher, rule: Rule) -> Self {
        Self {
            matcher,
            rule,
            players: vec![Player::at(1500, 0); n],
            pairs: Pairs::default(),
            stats: Metrics::new(n),
            divisions: crate::divisions::Comparison::selected(n),
            decks: None,
            trace: None,
        }
    }
    fn game(&mut self, tickets: [Ticket; 2], now: u32, r: &Round) {
        let ids = tickets.map(|t| t.id);
        let [a, b] = ids.map(|id| &r.pilots[id]);
        let diff = a.skill + a.deck - b.skill - b.deck;
        let p = r.odds.at(diff, 0.04);
        let roll = uniform(r.seed, r.slot, ids);
        let won = if ids[0] < ids[1] {
            roll < p
        } else {
            roll >= r.odds.at(-diff, 0.04)
        };
        let repeats = self.pairs.prior(ids[0], ids[1]);
        if let Some(field) = &mut self.decks {
            let gap = self.players[ids[0]].record.elo - self.players[ids[1]].record.elo;
            field.play(
                (now / DAY / u32::from(SEASON)) as usize,
                a.format,
                ids,
                gap,
                won,
            );
        }
        let players = self
            .players
            .get_disjoint_mut(ids)
            .expect("distinct accounts");
        let result = settle(
            players,
            Encounter {
                ids,
                now,
                repeats,
                a_won: won,
            },
            self.rule,
            r.odds,
        );
        self.pairs.played(ids[0], ids[1], now);
        self.stats.game(tickets, now, result, p, won, repeats > 0);
        if let Some(trace) = &mut self.trace {
            trace.push(Played {
                ids,
                now,
                won,
                elo: ids.map(|id| self.players[id].record.elo),
            });
        }
    }
    fn run_round(&mut self, r: &Round) {
        let Some(first) = r.tickets.first() else {
            return;
        };
        let mut queues: [Queue; 2] = std::array::from_fn(|_| Queue::default());
        let mut now = first.at;
        let mut arrivals = r.tickets.iter().peekable();
        loop {
            self.pairs.expire(now);
            while let Some(&&ticket) = arrivals.peek().filter(|t| t.at <= now) {
                self.stats.attempts += 1;
                let p = &r.pilots[ticket.id];
                self.stats.elite_attempts += u64::from(p.skill >= r.elite);
                queues[p.format].admit(ticket);
                arrivals.next();
            }
            for q in &mut queues {
                q.tick(now, |t| {
                    self.stats.timeout(t, r.pilots[t.id].skill >= r.elite);
                });
                while let Some(pair) =
                    q.take(now, self.matcher, self.rule, &self.players, &self.pairs)
                {
                    self.game(pair, now, r);
                }
            }
            if queues.iter().all(Queue::empty) {
                let Some(next) = arrivals.peek() else {
                    break;
                };
                now = next.at;
            } else {
                now += 5;
            }
        }
    }
}
fn simulate(case: Case, seed: u64) -> Vec<String> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut pilots = pilots(case, &mut rng);
    let skills: Vec<_> = pilots.iter().map(|p| p.skill).collect();
    let mut sorted = skills.clone();
    sorted.sort_unstable();
    let elite = sorted[case.n * 9 / 10];
    let deck_trial = case.name == "deck-cycle";
    let mut worlds: Vec<_> = if deck_trial {
        let mut paired: Vec<_> = (0..2)
            .map(|_| World::new(case.n, Matcher::Balanced, Rule::Evidence))
            .collect();
        let formats: Vec<_> = pilots.iter().map(|p| p.format).collect();
        paired[0].decks = Some(super::deck_cycle::Field::new(&formats));
        for w in &mut paired {
            w.trace = Some(Vec::new());
        }
        paired
    } else {
        POLICIES
            .into_iter()
            .map(|(m, r)| World::new(case.n, m, r))
            .collect()
    };
    let mut original_decks = Vec::new();
    let odds = Odds::new();
    let mut tickets = Vec::with_capacity(case.n);
    for day in 0..DAYS {
        if day % SEASON == 0 {
            for p in &mut pilots {
                p.deck = normal(&mut rng, 60.0);
            }
            if deck_trial {
                if day == 0 {
                    original_decks = pilots.iter().map(|p| p.deck).collect();
                } else {
                    worlds[0]
                        .decks
                        .as_mut()
                        .expect("deck comparison")
                        .close_season();
                }
                if day == 3 * SEASON {
                    for (p, original) in pilots.iter_mut().zip(&original_decks) {
                        p.deck = *original;
                    }
                }
            }
        }
        for slot in 0..10 {
            arrivals(&mut tickets, &pilots, case, day, slot, &mut rng);
            let round = Round {
                tickets: &tickets,
                pilots: &pilots,
                elite,
                odds: &odds,
                seed,
                slot: u32::from(day) * 10 + u32::from(slot),
            };
            for w in &mut worlds {
                w.run_round(&round);
            }
            if deck_trial {
                assert_eq!(
                    worlds[0].trace, worlds[1].trace,
                    "pair, time, outcome and both player ratings"
                );
                for w in &mut worlds {
                    w.trace.as_mut().expect("traced").clear();
                }
            }
        }
        for w in &mut worlds {
            w.stats.publish(&w.players, day, &mut w.divisions);
        }
    }
    if let Some(field) = worlds[0].decks.as_ref() {
        assert_eq!(field.max_error, 0);
        assert_eq!(worlds[0].stats.tiers, worlds[1].stats.tiers);
        return vec![format!(
            "{seed},{},{},{},{},0",
            case.n, field.matches, field.returns, field.max_error
        )];
    }
    worlds
        .into_iter()
        .map(|w| {
            format!(
                "{},{seed},{},{},{},{}",
                case.name,
                case.n,
                w.matcher.name(),
                w.rule.name(),
                w.stats.report(&w.players, &skills)
            )
        })
        .collect()
}
pub(super) fn deck_trials() {
    println!(
        "seed,accounts,matches_checked,restored_versions,max_restore_error,player_trace_differences"
    );
    let case = Case {
        name: "deck-cycle",
        n: 160,
        span: 1800,
        seeds: 6,
    };
    for seed in 91000..91000 + case.seeds {
        for row in simulate(case, seed) {
            println!("{row}");
        }
    }
}
pub(super) fn run(selection: Option<&str>) {
    println!("{HEADER}");
    let mut jobs = Vec::new();
    for (i, c) in CASES.into_iter().enumerate() {
        if selection.is_some_and(|s| s != c.name && s != "pilot") {
            continue;
        }
        if selection == Some("pilot") && c.name != "town" {
            continue;
        }
        let seeds = if selection == Some("pilot") {
            1
        } else {
            c.seeds + 3
        };
        for offset in 0..seeds {
            let stream = if offset < c.seeds {
                offset
            } else {
                50 + offset - c.seeds
            };
            jobs.push((c, 71000 + (i as u64) * 100 + stream));
        }
    }
    if jobs.is_empty() {
        eprintln!("Unknown population. Choose duel, night, town, city, peak or pilot.");
        std::process::exit(2);
    }
    for batch in jobs.chunks(4) {
        std::thread::scope(|scope| {
            let runs: Vec<_> = batch
                .iter()
                .map(|&(case, seed)| scope.spawn(move || simulate(case, seed)))
                .collect();
            for run in runs {
                for row in run.join().expect("population completed") {
                    println!("{row}");
                }
            }
        });
    }
}
