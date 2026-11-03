//! Counts recurring mistakes of the evaluated seat over many games.
//!
//! Two kinds so far, both read straight off the question and the answer:
//! - targets: what each spell or ability was aimed at, the seat's own
//!   objects (or itself) against everything else;
//! - unspent turns: passing priority in its own second main phase with the
//!   stack empty while a card in hand could still be cast with the mana it
//!   had (`LegalActions::castable`), or a land still be played.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{Phase, PlayerView};

use crate::game::{Decision, Event, Game};
use crate::losses::{self, Which};
use crate::names::Names;

/// Where a target answer pointed.
#[derive(Debug, Default, Clone, Copy)]
struct Aims {
    own: usize,
    other: usize,
}

/// What the games showed.
#[derive(Debug, Default)]
struct Patterns {
    games: usize,
    decisions: usize,
    own_turns: usize,
    aims: BTreeMap<String, Aims>,
    unspent_turns: usize,
    unspent_cards: BTreeMap<String, usize>,
    unplayed_lands: usize,
}

impl Patterns {
    fn merge(&mut self, other: Self) {
        self.games += other.games;
        self.decisions += other.decisions;
        self.own_turns += other.own_turns;
        for (source, aims) in other.aims {
            let entry = self.aims.entry(source).or_default();
            entry.own += aims.own;
            entry.other += aims.other;
        }
        self.unspent_turns += other.unspent_turns;
        for (card, count) in other.unspent_cards {
            *self.unspent_cards.entry(card).or_default() += count;
        }
        self.unplayed_lands += other.unplayed_lands;
    }

    fn decide(&mut self, seat: PlayerId, decision: &Decision<'_>) {
        if decision.source == "Forced" {
            return;
        }
        self.decisions += 1;
        let table = &decision.views[usize::from(seat.get())];
        match (decision.pending, decision.action) {
            (_, PlayerAction::ChooseTargets { objects, players }) => {
                let source = decision
                    .asked
                    .and_then(|view| view.targeting.as_ref())
                    .map_or_else(|| "?".to_owned(), |targeting| targeting.source.name.clone());
                let own = players.contains(&seat)
                    || objects.iter().any(|id| controlled_by(table, *id, seat));
                let aims = self.aims.entry(source).or_default();
                if own {
                    aims.own += 1;
                } else {
                    aims.other += 1;
                }
            }
            (Pending::Priority { legal, .. }, PlayerAction::PassPriority)
                if table.active == seat
                    && table.phase == Phase::SecondMain
                    && table.stack.is_empty() =>
            {
                if !legal.castable.is_empty() {
                    self.unspent_turns += 1;
                    let names = Names::of(decision.views);
                    for card in &legal.castable {
                        *self.unspent_cards.entry(names.name(*card)).or_default() += 1;
                    }
                }
                if !legal.lands.is_empty() {
                    self.unplayed_lands += 1;
                }
            }
            _ => {}
        }
    }

    fn print(&self) {
        println!(
            "{} games, {} decisions not forced, {} own turns",
            self.games, self.decisions, self.own_turns
        );
        println!("\nTargets (source: aimed at own objects or self / elsewhere), most own first:");
        let mut aims: Vec<_> = self.aims.iter().filter(|(_, aims)| aims.own > 0).collect();
        aims.sort_by_key(|(source, aims)| (std::cmp::Reverse(aims.own), (*source).clone()));
        for (source, aims) in aims.iter().take(40) {
            println!(
                "  {:>4} own / {:>4} elsewhere  {source}",
                aims.own, aims.other
            );
        }
        println!(
            "\nOwn second main phase passed with a castable card in hand: {} of {} own turns",
            self.unspent_turns, self.own_turns
        );
        println!("…with a land still playable: {}", self.unplayed_lands);
        let mut cards: Vec<_> = self.unspent_cards.iter().collect();
        cards.sort_by_key(|(card, count)| (std::cmp::Reverse(**count), (*card).clone()));
        println!("Cards left castable most often:");
        for (card, count) in cards.iter().take(30) {
            println!("  {count:>4}  {card}");
        }
    }
}

/// Whether `id` is on `seat`'s side of the table (battlefield or stack).
fn controlled_by(view: &PlayerView, id: baylee_core::ids::ObjectId, seat: PlayerId) -> bool {
    view.battlefield
        .iter()
        .chain(&view.stack)
        .any(|object| object.id == id && object.controller == seat)
}

/// The patterns of one game.
fn game(run: &Path, deal: u64) -> anyhow::Result<Patterns> {
    let record = Game::load(run, deal)?;
    let seat = record.seat;
    let mut patterns = Patterns {
        games: 1,
        ..Patterns::default()
    };
    record.replay(false, |event| {
        match event {
            Event::Turn { views, .. } if views[usize::from(seat.get())].active == seat => {
                patterns.own_turns += 1;
            }
            Event::Decision(decision) => patterns.decide(seat, &decision),
            _ => {}
        }
        Ok(())
    })?;
    Ok(patterns)
}

/// Counts the patterns over the deals of `summary` that `which` selects,
/// replaying their games from `run` on `threads` threads.
pub(super) fn count(
    run: &Path,
    summary: &Path,
    which: Which,
    threads: usize,
) -> anyhow::Result<()> {
    let deals = losses::deals(summary, which)?;
    let next = AtomicUsize::new(0);
    let total = Mutex::new(Patterns::default());
    let failures = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                while let Some(deal) = deals.get(next.fetch_add(1, Ordering::Relaxed)) {
                    match game(run, deal.number) {
                        Ok(patterns) => total.lock().expect("patterns lock").merge(patterns),
                        Err(error) => failures
                            .lock()
                            .expect("failures lock")
                            .push(format!("deal {}: {error:#}", deal.number)),
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("failures lock");
    for failure in &failures {
        eprintln!("[gamelog4] {failure}");
    }
    println!("{which:?} deals of {}", summary.display());
    total.into_inner().expect("patterns lock").print();
    anyhow::ensure!(
        failures.is_empty(),
        "{} games could not be replayed",
        failures.len()
    );
    Ok(())
}
