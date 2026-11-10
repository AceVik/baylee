//! `gamelog4 react`: what the evaluated seat did with its instant-speed
//! answers, counted game by game. A count for a reader, never a rule.
//!
//! - **Counter windows**: the seat holds priority with a spell or ability of
//!   the other side on top of the stack and a counterspell in hand it can pay
//!   for and the tactical layer reads as worth casting
//!   (`reading::reachable_spells`, `reading::counters`, `reading::worth_casting`).
//!   Each such top object is *eligible*; it is *countered* when one of the
//!   seat's own counterspells on the stack names it, and *held* otherwise.
//!   `layer` counts the windows the `React` group itself reads
//!   (`reading::react_window` window 0), which sees a spell's single effect
//!   list only: a modal counter is outside it.
//! - **Counters cast**: every counterspell of the seat put on the stack with a
//!   stack object among its targets, with the turn it was cast on and what it
//!   names: the kind and mana value, whose it is, and whether it makes mana.
//!   A target is *poor* when it is the seat's own or a teammate's, makes mana,
//!   or is a spell of at most `--cheap` mana value.
//! - **Their attack, their end step**: the seat passed with an instant that
//!   is not a counter usable in that window (`reading::react_window` 1 and 2),
//!   once per turn and window.
//! - **Instants**: cast at the other side's end step, elsewhere in their turn,
//!   in the seat's own turn; discarded to hand size; still in hand at the end.
//!
//! A record is omniscient; every reading here is the seat's own view.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use baylee_core::ids::{ObjectId, PlayerId, TargetRef};
use baylee_core::types::TypeSet;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
use baylee_train::netplay4::reading;
use baylee_view::{PlayerView, PublicObject, StackItem, Step};
use serde_json::{Map, Value, json};

use crate::game::{Decision, Event, Game};
use crate::losses::{self, Which};

/// What a counter names, or what sat on top of the stack.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Target {
    name: String,
    /// `ability`, or the spell's first type of planeswalker, creature,
    /// battle, sorcery, instant, artifact, enchantment.
    kind: &'static str,
    mana_value: u32,
    /// `theirs`, `own` or `team`.
    side: &'static str,
    /// It has a mana ability, or is a land: a rock, a dork, a ramp permanent.
    makes_mana: bool,
}

impl Target {
    fn of(object: &PublicObject, seat: PlayerId, side: &[PlayerId]) -> Self {
        let kind = if matches!(object.stack_item, Some(StackItem::Ability { .. })) {
            "ability"
        } else {
            [
                (TypeSet::PLANESWALKER, "planeswalker"),
                (TypeSet::CREATURE, "creature"),
                (TypeSet::BATTLE, "battle"),
                (TypeSet::SORCERY, "sorcery"),
                (TypeSet::INSTANT, "instant"),
                (TypeSet::ARTIFACT, "artifact"),
                (TypeSet::ENCHANTMENT, "enchantment"),
            ]
            .into_iter()
            .find(|(types, _)| object.types.contains(*types))
            .map_or("other", |(_, word)| word)
        };
        let makes_mana = kind != "ability"
            && object.card.is_some_and(|identity| {
                baylee_cards::by_index(identity.index).is_some_and(|def| {
                    baylee_train::reactivity::face_makes_mana(def, usize::from(identity.face))
                })
            });
        Self {
            name: object.name.to_string(),
            kind,
            mana_value: object.mana_value,
            side: if object.controller == seat {
                "own"
            } else if side.contains(&object.controller) {
                "team"
            } else {
                "theirs"
            },
            makes_mana,
        }
    }

    /// Why a counter at this is a poor use of it, if it is.
    fn poor(&self, cheap: u32) -> Option<&'static str> {
        if self.side != "theirs" {
            Some("own side")
        } else if self.makes_mana {
            Some("makes mana")
        } else if self.kind != "ability" && self.mana_value <= cheap {
            Some("cheap")
        } else {
            None
        }
    }

    fn json(&self, cheap: u32) -> Value {
        json!({"name": self.name, "kind": self.kind, "mana_value": self.mana_value,
            "side": self.side, "makes_mana": self.makes_mana, "poor": self.poor(cheap)})
    }
}

/// A mana value as a histogram's key: 0 to 5, then `6+`.
fn mana_bucket(value: u32) -> String {
    if value >= 6 {
        "6+".to_owned()
    } else {
        value.to_string()
    }
}

/// One counterspell of the seat on the stack.
struct CounterCast {
    their_turn: bool,
    targets: Vec<Target>,
}

/// One game's counts.
#[derive(Default)]
struct Counts {
    counter_windows: u64,
    layer_counter_windows: u64,
    counter_window_passes: u64,
    eligible: BTreeMap<ObjectId, Target>,
    countered: BTreeSet<ObjectId>,
    counters: Vec<CounterCast>,
    held_their_attack: u64,
    held_their_end: u64,
    instants_their_end: u64,
    instants_their_other: u64,
    instants_own: u64,
    instants_discarded: u64,
    counters_discarded: u64,
    instants_held_at_end: u64,
    counters_held_at_end: u64,
}

/// One game's reader.
struct Reader {
    seat: PlayerId,
    side: Vec<PlayerId>,
    /// The seat's spells on the stack already read.
    read: BTreeSet<ObjectId>,
    /// Windows 1 and 2 already counted, by turn.
    held: BTreeSet<(u32, usize)>,
    /// The seat's instants and counters in hand at the last view.
    in_hand: (u64, u64),
    counts: Counts,
}

impl Reader {
    fn new(seat: PlayerId, side: Vec<PlayerId>) -> Self {
        Self {
            seat,
            side,
            read: BTreeSet::new(),
            held: BTreeSet::new(),
            in_hand: (0, 0),
            counts: Counts::default(),
        }
    }

    fn their_turn(&self, view: &PlayerView) -> bool {
        !self.side.contains(&view.active)
    }

    /// Reads the seat's counterspells on the stack and its hand.
    fn watch(&mut self, view: &PlayerView) {
        for object in &view.stack {
            if object.controller != self.seat
                || !matches!(object.stack_item, Some(StackItem::Spell))
                || object.targets.is_empty()
                || self.read.contains(&object.id)
            {
                continue;
            }
            self.read.insert(object.id);
            if !reading::counters(view, object.id) {
                continue;
            }
            let named: Vec<&PublicObject> = object
                .targets
                .iter()
                .filter_map(|target| match target {
                    TargetRef::Object(reference) => view
                        .stack
                        .iter()
                        .find(|item| item.id == reference.object && item.id != object.id),
                    TargetRef::Player(_) => None,
                })
                .collect();
            if named.is_empty() {
                // A modal counter cast for another mode.
                continue;
            }
            self.counts
                .countered
                .extend(named.iter().map(|item| item.id));
            self.counts.counters.push(CounterCast {
                their_turn: self.their_turn(view),
                targets: named
                    .into_iter()
                    .map(|item| Target::of(item, self.seat, &self.side))
                    .collect(),
            });
        }
        let instants = view
            .hand
            .iter()
            .filter(|card| card.types.contains(TypeSet::INSTANT));
        let counters = view
            .hand
            .iter()
            .filter(|card| reading::counters(view, card.id));
        self.in_hand = (instants.count() as u64, counters.count() as u64);
    }

    /// A priority decision: the windows it stands in, and what was cast.
    fn priority(&mut self, view: &PlayerView, legal: &LegalActions, action: &PlayerAction) {
        let passed = matches!(action, PlayerAction::PassPriority);
        if view.owed.is_none()
            && let Some(top) = view.top_of_stack()
            && !self.side.contains(&top.controller)
        {
            let in_hand = |id: &ObjectId| view.hand.iter().any(|card| card.id == *id);
            let counters: Vec<ObjectId> = reading::reachable_spells(view, legal)
                .into_iter()
                .filter(|id| {
                    in_hand(id) && reading::counters(view, *id) && reading::worth_casting(view, *id)
                })
                .collect();
            if !counters.is_empty() {
                self.counts.counter_windows += 1;
                self.counts.counter_window_passes += u64::from(passed);
                self.counts
                    .eligible
                    .entry(top.id)
                    .or_insert_with(|| Target::of(top, self.seat, &self.side));
                if reading::react_window(view, legal).is_some_and(|(window, _)| window == 0) {
                    self.counts.layer_counter_windows += 1;
                }
            }
        }
        if passed {
            if let Some((window @ (1 | 2), _)) = reading::react_window(view, legal)
                && self.held.insert((view.turn, window))
            {
                if window == 1 {
                    self.counts.held_their_attack += 1;
                } else {
                    self.counts.held_their_end += 1;
                }
            }
            return;
        }
        let PlayerAction::CastSpell { card } = action else {
            return;
        };
        let instant = view
            .hand
            .iter()
            .any(|hand| hand.id == *card && hand.types.contains(TypeSet::INSTANT));
        if !instant {
            return;
        }
        if !self.their_turn(view) {
            self.counts.instants_own += 1;
        } else if view.step == Step::End {
            self.counts.instants_their_end += 1;
        } else {
            self.counts.instants_their_other += 1;
        }
    }

    fn decide(&mut self, decision: &Decision<'_>) {
        let fallback = &decision.views[usize::from(self.seat.get())];
        let view = decision.asked.unwrap_or(fallback);
        match (decision.pending, decision.action) {
            (Pending::Priority { legal, .. }, action) => self.priority(view, legal, action),
            (Pending::DiscardChoice { .. }, PlayerAction::ChooseObjects { objects }) => {
                for id in objects {
                    if view
                        .hand
                        .iter()
                        .any(|card| card.id == *id && card.types.contains(TypeSet::INSTANT))
                    {
                        self.counts.instants_discarded += 1;
                    }
                    if reading::counters(view, *id) {
                        self.counts.counters_discarded += 1;
                    }
                }
            }
            _ => {}
        }
    }
}

/// One game's row.
fn game(run: &Path, deal: &losses::Deal, cheap: u32) -> anyhow::Result<Value> {
    let record = Game::load(run, deal.number)?;
    let seat = record.seat;
    let won = record.row["outcome"]["kind"].as_str() == Some("won")
        && record.row["outcome"]["seat"].as_u64() == Some(u64::from(seat.get()));
    let mut reader = Reader::new(seat, record.side());
    record.replay(false, |event| {
        match event {
            Event::Turn { views, .. } | Event::Other { views } => {
                reader.watch(&views[usize::from(seat.get())]);
            }
            Event::Decision(decision) => {
                reader.watch(&decision.views[usize::from(seat.get())]);
                reader.decide(&decision);
            }
            Event::End { .. } => {
                reader.counts.instants_held_at_end = reader.in_hand.0;
                reader.counts.counters_held_at_end = reader.in_hand.1;
            }
            Event::Log { .. } => {}
        }
        Ok(())
    })?;
    Ok(row(deal, won, &reader.counts, cheap))
}

fn row(deal: &losses::Deal, won: bool, counts: &Counts, cheap: u32) -> Value {
    let seat = usize::try_from(deal.seat).unwrap_or(0);
    let held: Vec<&Target> = counts
        .eligible
        .iter()
        .filter(|(id, _)| !counts.countered.contains(id))
        .map(|(_, target)| target)
        .collect();
    let mut by_kind: BTreeMap<&str, [u64; 2]> = BTreeMap::new();
    let mut by_mana: BTreeMap<String, [u64; 2]> = BTreeMap::new();
    for (id, target) in &counts.eligible {
        let column = usize::from(!counts.countered.contains(id));
        by_kind.entry(target.kind).or_default()[column] += 1;
        by_mana.entry(mana_bucket(target.mana_value)).or_default()[column] += 1;
    }
    let targets: Vec<&Target> = counts
        .counters
        .iter()
        .flat_map(|cast| &cast.targets)
        .collect();
    let poor: BTreeMap<&str, u64> = targets.iter().filter_map(|target| target.poor(cheap)).fold(
        BTreeMap::new(),
        |mut tally, why| {
            *tally.entry(why).or_default() += 1;
            tally
        },
    );
    json!({
        "deal": deal.number,
        "pilot": deal.decks.get(seat),
        "opponents": deal.decks.iter().enumerate().filter(|(index, _)| *index != seat)
            .map(|(_, deck)| deck).collect::<Vec<_>>(),
        "won": won,
        "counter_windows": counts.counter_windows,
        "layer_counter_windows": counts.layer_counter_windows,
        "counter_window_passes": counts.counter_window_passes,
        "eligible": counts.eligible.len(),
        "eligible_countered": counts.eligible.len() - held.len(),
        "eligible_held": held.len(),
        "eligible_by_kind": by_kind.iter().map(|(kind, [countered, held])|
            (kind.to_string(), json!({"countered": countered, "held": held}))).collect::<Map<_, _>>(),
        "eligible_by_mana_value": by_mana.iter().map(|(bucket, [countered, held])|
            (bucket.clone(), json!({"countered": countered, "held": held}))).collect::<Map<_, _>>(),
        "held_targets": held.iter().map(|target| target.json(cheap)).collect::<Vec<_>>(),
        "counters_cast": counts.counters.len(),
        "counters_their_turn": counts.counters.iter().filter(|cast| cast.their_turn).count(),
        "counters_own_turn": counts.counters.iter().filter(|cast| !cast.their_turn).count(),
        "counter_targets": targets.iter().map(|target| target.json(cheap)).collect::<Vec<_>>(),
        "counter_targets_poor": poor.values().sum::<u64>(),
        "counter_targets_poor_by_reason": poor,
        "held_their_attack": counts.held_their_attack,
        "held_their_end": counts.held_their_end,
        "instants_their_end": counts.instants_their_end,
        "instants_their_other": counts.instants_their_other,
        "instants_own": counts.instants_own,
        "instants_discarded": counts.instants_discarded,
        "counters_discarded": counts.counters_discarded,
        "instants_held_at_end": counts.instants_held_at_end,
        "counters_held_at_end": counts.counters_held_at_end,
    })
}

/// The counts summed over games: every number, and every histogram by key.
fn totals(rows: &[Value]) -> Value {
    fn add(into: &mut Map<String, Value>, key: &str, value: &Value) {
        match value {
            Value::Number(number) => {
                let sum = into.get(key).and_then(Value::as_u64).unwrap_or(0)
                    + number.as_u64().unwrap_or(0);
                into.insert(key.to_owned(), json!(sum));
            }
            Value::Object(inner) => {
                let entry = into
                    .entry(key.to_owned())
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Value::Object(entry) = entry {
                    for (inner_key, inner_value) in inner {
                        add(entry, inner_key, inner_value);
                    }
                }
            }
            _ => {}
        }
    }
    let mut total = Map::new();
    for row in rows {
        for (key, value) in row.as_object().into_iter().flatten() {
            if key != "deal" {
                add(&mut total, key, value);
            }
        }
    }
    total.insert("games".to_owned(), json!(rows.len()));
    total.insert(
        "won".to_owned(),
        json!(rows.iter().filter(|row| row["won"] == true).count()),
    );
    let mut targets: BTreeMap<String, u64> = BTreeMap::new();
    for row in rows {
        for target in row["counter_targets"].as_array().into_iter().flatten() {
            let key = format!(
                "{} {} mv {}",
                target["side"].as_str().unwrap_or("?"),
                target["kind"].as_str().unwrap_or("?"),
                mana_bucket(u32::try_from(target["mana_value"].as_u64().unwrap_or(0)).unwrap_or(0))
            );
            *targets.entry(key).or_default() += 1;
        }
    }
    total.insert("counter_targets_by_kind".to_owned(), json!(targets));
    Value::Object(total)
}

/// The rows the table prints, by their key in [`totals`].
const PRINTED: [(&str, &str); 19] = [
    ("games", "games"),
    ("won", "won"),
    (
        "counter_windows",
        "counter windows (their object on top, counter reachable)",
    ),
    ("layer_counter_windows", "  of them read by the React group"),
    ("counter_window_passes", "  passed in them"),
    ("eligible", "eligible objects of theirs"),
    ("eligible_countered", "  countered"),
    (
        "eligible_held",
        "  held (resolved past a reachable counter)",
    ),
    ("counters_cast", "counters cast"),
    ("counters_their_turn", "  on their turn"),
    ("counters_own_turn", "  on our turn"),
    ("counter_targets_poor", "  at a poor target"),
    (
        "held_their_attack",
        "passed their attack with a usable instant",
    ),
    (
        "held_their_end",
        "passed their end step with a usable instant",
    ),
    ("instants_their_end", "instants cast at their end step"),
    (
        "instants_their_other",
        "instants cast elsewhere in their turn",
    ),
    ("instants_own", "instants cast on our turn"),
    ("instants_discarded", "instants discarded to hand size"),
    ("instants_held_at_end", "instants in hand at the game's end"),
];

fn per_game(total: &Value, key: &str) -> String {
    let games = total["games"].as_u64().unwrap_or(0).max(1);
    let count = total[key].as_u64().unwrap_or(0);
    #[allow(clippy::cast_precision_loss, reason = "counts of a few thousand games")]
    let rate = count as f64 / games as f64;
    format!("{count} ({rate:.2})")
}

fn print(labels: &[String], totals: &[Value]) {
    println!("| count (per game) | {} |", labels.join(" | "));
    println!("| --- |{}", " --- |".repeat(labels.len()));
    for (key, label) in PRINTED {
        let cells: Vec<String> = totals.iter().map(|total| per_game(total, key)).collect();
        println!("| {label} | {} |", cells.join(" | "));
    }
    for (label, total) in labels.iter().zip(totals) {
        println!("\n{label}: eligible by kind {}", total["eligible_by_kind"]);
        println!(
            "{label}: eligible by mana value {}",
            total["eligible_by_mana_value"]
        );
        println!(
            "{label}: counter targets {}",
            total["counter_targets_by_kind"]
        );
        println!(
            "{label}: poor targets by reason {}",
            total["counter_targets_poor_by_reason"]
        );
    }
}

/// Counts every game of `runs` (each a record directory with its summary)
/// whose net piloted `pilot`, if given, prints them side by side and with
/// `out` writes every game's row and the totals there.
pub(super) fn count(
    runs: &[(PathBuf, PathBuf)],
    pilot: Option<&str>,
    cheap: u32,
    threads: usize,
    out: Option<&Path>,
) -> anyhow::Result<()> {
    let mut reports = Vec::new();
    let mut labels = Vec::new();
    let mut failed = 0;
    for (run, summary) in runs {
        let deals: Vec<losses::Deal> = losses::deals(summary, Which::All)?
            .into_iter()
            .filter(|deal| {
                pilot.is_none_or(|pilot| {
                    usize::try_from(deal.seat)
                        .ok()
                        .and_then(|seat| deal.decks.get(seat))
                        .is_some_and(|deck| deck == pilot)
                })
            })
            .collect();
        let next = AtomicUsize::new(0);
        let rows = Mutex::new(Vec::new());
        let failures = Mutex::new(Vec::new());
        std::thread::scope(|scope| {
            for _ in 0..threads.max(1) {
                scope.spawn(|| {
                    while let Some(deal) = deals.get(next.fetch_add(1, Ordering::Relaxed)) {
                        match game(run, deal, cheap) {
                            Ok(row) => rows.lock().expect("rows lock").push(row),
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
            eprintln!("[gamelog4] {}: {failure}", run.display());
        }
        failed += failures.len();
        let mut rows = rows.into_inner().expect("rows lock");
        rows.sort_by_key(|row| row["deal"].as_u64());
        let total = totals(&rows);
        labels.push(summary.file_stem().map_or_else(
            || run.display().to_string(),
            |stem| stem.to_string_lossy().into_owned(),
        ));
        reports.push(json!({"run": run, "summary": summary, "pilot_deck": pilot,
            "cheap_mana_value": cheap, "total": total, "games": rows,
            "failed": failures}));
    }
    let totals: Vec<Value> = reports
        .iter()
        .map(|report| report["total"].clone())
        .collect();
    print(&labels, &totals);
    if let Some(path) = out {
        baylee_train_ops::files::atomic_json(path, &json!({"runs": reports}))?;
    }
    anyhow::ensure!(failed == 0, "{failed} games could not be replayed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poor_target_says_why() {
        let target = |side, kind, mana_value, makes_mana| Target {
            name: String::new(),
            kind,
            mana_value,
            side,
            makes_mana,
        };
        assert_eq!(
            target("own", "creature", 5, false).poor(1),
            Some("own side")
        );
        assert_eq!(
            target("team", "sorcery", 4, false).poor(1),
            Some("own side")
        );
        assert_eq!(
            target("theirs", "artifact", 2, true).poor(1),
            Some("makes mana")
        );
        assert_eq!(target("theirs", "instant", 1, false).poor(1), Some("cheap"));
        assert_eq!(target("theirs", "ability", 0, false).poor(1), None);
        assert_eq!(target("theirs", "creature", 3, false).poor(1), None);
    }

    #[test]
    fn totals_add_numbers_and_histograms_by_key() {
        let rows = [
            json!({"deal": 1, "won": true, "eligible": 2,
                "eligible_by_kind": {"creature": {"countered": 1, "held": 1}},
                "counter_targets": [{"side": "theirs", "kind": "creature", "mana_value": 7}]}),
            json!({"deal": 2, "won": false, "eligible": 3,
                "eligible_by_kind": {"creature": {"countered": 0, "held": 2},
                    "ability": {"countered": 1, "held": 0}},
                "counter_targets": []}),
        ];
        let total = totals(&rows);
        assert_eq!(total["games"], 2);
        assert_eq!(total["won"], 1);
        assert_eq!(total["eligible"], 5);
        assert_eq!(total["eligible_by_kind"]["creature"]["held"], 3);
        assert_eq!(total["eligible_by_kind"]["ability"]["countered"], 1);
        assert_eq!(total["counter_targets_by_kind"]["theirs creature mv 6+"], 1);
        assert!(total.get("deal").is_none());
        assert_eq!(per_game(&total, "eligible"), "5 (2.50)");
    }

    #[test]
    fn mana_values_from_six_share_a_bucket() {
        assert_eq!(mana_bucket(0), "0");
        assert_eq!(mana_bucket(5), "5");
        assert_eq!(mana_bucket(9), "6+");
    }
}
