//! A measurement, not a check: whether team play is exercised at a real
//! table, and how often it changes an answer. Run by hand:
//! `cargo nextest run -p baylee-gamehost --run-ignored only --no-capture
//! team_play_is_exercised`.
//!
//! Two-against-two house tables on the house decks, every chair `sharp`.
//! At every question a house chair is asked, the table's own answer is set
//! beside the frozen reference's in the same chair, on the same view and
//! context; the game goes on with the table's. Counted per deck the asked
//! chair plays.

use super::*;
use crate::session::table::Table;
use baylee_ai::{HeuristicAgent, policy_seed};
use std::collections::BTreeMap;

const DECKS: [&str; 5] = ["allytifact", "victory", "maik", "weltenbaum", "schwarzrand"];

fn deck(name: &str) -> baylee_cards::decks::LoadedDeck {
    let path = format!("{}/../../data/decks/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    baylee_cards::decks::house_deck(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[derive(Default, Debug)]
struct Count {
    decisions: u32,
    shown: u32,
    differs: u32,
    differs_shown: u32,
}

#[test]
#[ignore = "a measurement, run by hand"]
fn team_play_is_exercised() {
    let tables: u64 = std::env::var("BAYLEE_TEAM_TABLES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(40);
    let loaded: Vec<_> = DECKS.iter().map(|name| deck(name)).collect();
    let mut per_deck: BTreeMap<&str, Count> = BTreeMap::new();
    let teams = [Some(1), Some(2), Some(1), Some(2)];
    for seed in 0..tables {
        let at = |i: u64| usize::try_from((seed + i) % 5).expect("small");
        let chosen = [at(0), at(1), at(2), at(3)];
        let refs: Vec<_> = chosen.iter().map(|&i| &loaded[i]).collect();
        let mut preset = baylee_cards::decks::preset_for_all(seed + 1, &refs);
        for (spec, team) in preset.seats.iter_mut().zip(teams) {
            spec.controller = SeatController::Ai(AIProfile::SHARP);
            spec.team = team;
        }
        let mut session = Session::new(&preset).expect("the table starts");
        let game = format!("team-exercise/{seed}");
        session.describe(game.clone(), (0..4).map(|s| s.to_string()).collect());
        session.discard_frames();
        let frozen: Vec<SeatKind> = (0..4)
            .map(|i| {
                SeatKind::Ai(
                    HeuristicAgent::new(AIProfile::SHARP_2026_11)
                        .with_teams(teams.to_vec())
                        .with_seed(policy_seed(&game, seat_byte(i))),
                )
            })
            .collect();
        for _ in 0..4_000 {
            if matches!(session.pending(), Pending::GameOver(_)) {
                break;
            }
            for seat in session.engine.awaited().iter() {
                let Some(pending) = session.engine.pending_for(seat).cloned() else {
                    continue;
                };
                let table = session.table();
                let view = table.house_view(seat, &pending);
                let shown = view.shared_hands.iter().any(|hand| !hand.public);
                let live = table.house_answer(seat, &pending);
                let reference = Table {
                    seats: &frozen,
                    ..session.table()
                }
                .house_answer(seat, &pending);
                let count = per_deck
                    .entry(DECKS[chosen[usize::from(seat.get())]])
                    .or_default();
                count.decisions += 1;
                count.shown += u32::from(shown);
                let differs = live != reference;
                count.differs += u32::from(differs);
                count.differs_shown += u32::from(differs && shown);
            }
            let _ = session.pump_at_most(1);
        }
    }
    let mut total = Count::default();
    for (deck, count) in &per_deck {
        eprintln!(
            "{deck:12} decisions {:6} with a teammate's hand {:6} ({:4.1} %) answer changed {:5} ({:4.2} %), {} of them with a hand shown",
            count.decisions,
            count.shown,
            100.0 * f64::from(count.shown) / f64::from(count.decisions.max(1)),
            count.differs,
            100.0 * f64::from(count.differs) / f64::from(count.decisions.max(1)),
            count.differs_shown
        );
        total.decisions += count.decisions;
        total.shown += count.shown;
        total.differs += count.differs;
        total.differs_shown += count.differs_shown;
    }
    eprintln!("all          {total:?} over {tables} tables");
    assert!(
        total.shown > 0,
        "no house chair was ever shown a teammate's hand"
    );
}
