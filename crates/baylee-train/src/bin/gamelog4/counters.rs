//! The evaluated seat's counterspells and what they countered, over every
//! game of a record directory: a regression watch for a net that reacts
//! more, that it does not start countering at random.
//!
//! A counterspell is a spell of the seat's on the stack that the tactical
//! layer reads as one (`reading::is_counterspell_card`: its effects counter
//! a target spell or ability). It is read off the stack as the seat sees it
//! at its next question, whether its target was asked or the only one, with
//! the target the spell names: a spell's kind (its first card type) and
//! mana value, or an ability, or a player, and whose it is. A counter is on
//! a poor target when that target is the seat's own or a teammate's, or a
//! spell of mana value 1 or less. Whether the countered spell would have
//! done anything is not judged. A count, not a rule.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead as _, BufReader};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use baylee_core::ids::{ObjectId, PlayerId, TargetRef};
use baylee_core::types::TypeSet;
use baylee_engine::choice::PlayerAction;
use baylee_train::netplay4::reading;
use baylee_view::{PlayerView, PublicObject, StackItem};
use serde::Serialize;
use serde_json::{Value, json};

use crate::game::{Event, Game};

/// A spell this cheap or cheaper is a poor target for a counterspell.
const CHEAP: u32 = 1;

/// One counterspell the seat cast and what it was aimed at.
#[derive(Clone, Debug, Serialize)]
struct Counter {
    deal: u64,
    seed: Option<u64>,
    turn: u32,
    counterspell: String,
    /// Who answered the cast for the seat (`Net`, `Search`, `House`).
    source: String,
    target: String,
    /// The target's first card type, `ability`, `player`, or `none`.
    kind: String,
    mana_value: Option<u32>,
    /// `opponent`, `own`, `teammate` or `unknown`.
    whose: String,
    poor: bool,
}

/// A spell's kind: its first card type, in the order a reader names it.
fn kind(types: TypeSet) -> &'static str {
    [
        (TypeSet::CREATURE, "creature"),
        (TypeSet::PLANESWALKER, "planeswalker"),
        (TypeSet::INSTANT, "instant"),
        (TypeSet::SORCERY, "sorcery"),
        (TypeSet::ARTIFACT, "artifact"),
        (TypeSet::ENCHANTMENT, "enchantment"),
        (TypeSet::BATTLE, "battle"),
        (TypeSet::LAND, "land"),
    ]
    .into_iter()
    .find_map(|(t, name)| types.contains(t).then_some(name))
    .unwrap_or("other")
}

/// Whose `controller` is, seen from `seat`.
fn whose(seat: PlayerId, controller: PlayerId, teams: &[Option<u8>]) -> &'static str {
    let team = |p: PlayerId| teams.get(usize::from(p.get())).copied().flatten();
    if controller == seat {
        "own"
    } else if team(seat).is_some() && team(seat) == team(controller) {
        "teammate"
    } else {
        "opponent"
    }
}

/// What the counterspell `spell` names as its target, as `view` shows it:
/// name, kind, mana value, whose.
fn aimed(
    view: &PlayerView,
    spell: &PublicObject,
    teams: &[Option<u8>],
) -> (String, String, Option<u32>, String) {
    match spell.targets.first() {
        Some(TargetRef::Object(target)) => {
            match view.stack.iter().find(|o| o.id == target.object) {
                Some(object) => {
                    let (kind, mana_value) = match object.stack_item {
                        Some(StackItem::Ability { .. }) => ("ability".to_owned(), None),
                        _ => (kind(object.types).to_owned(), Some(object.mana_value)),
                    };
                    (
                        object.name.to_string(),
                        kind,
                        mana_value,
                        whose(view.seat, object.controller, teams).to_owned(),
                    )
                }
                None => (
                    "?".to_owned(),
                    "gone".to_owned(),
                    None,
                    "unknown".to_owned(),
                ),
            }
        }
        Some(TargetRef::Player(player)) => (
            format!("P{}", player.get() + 1),
            "player".to_owned(),
            None,
            whose(view.seat, *player, teams).to_owned(),
        ),
        None => (
            "-".to_owned(),
            "none".to_owned(),
            None,
            "unknown".to_owned(),
        ),
    }
}

/// The seat's counterspells in deal `deal` of `run`.
fn game(run: &Path, deal: u64, seed: Option<u64>) -> anyhow::Result<Vec<Counter>> {
    let game = Game::load(run, deal)?;
    let teams = game.teams();
    let seat = usize::from(game.seat.get());
    // Who answered each cast, by the card's handle (a card keeps it from
    // hand to stack), and the counterspells already counted while they
    // stay on the stack.
    let mut cast_by: BTreeMap<ObjectId, String> = BTreeMap::new();
    let mut counted: BTreeSet<ObjectId> = BTreeSet::new();
    let mut found = Vec::new();
    game.replay(false, |event| {
        let Event::Decision(decision) = event else {
            return Ok(());
        };
        let view = &decision.views[seat];
        counted.retain(|id| view.stack.iter().any(|o| o.id == *id));
        for spell in &view.stack {
            if spell.controller != view.seat
                || !matches!(spell.stack_item, None | Some(StackItem::Spell))
                || counted.contains(&spell.id)
                || !spell.card.is_some_and(reading::is_counterspell_card)
            {
                continue;
            }
            counted.insert(spell.id);
            let (target, kind, mana_value, whose) = aimed(view, spell, &teams);
            let poor = whose == "own"
                || whose == "teammate"
                || (kind != "ability" && mana_value.is_some_and(|mv| mv <= CHEAP));
            found.push(Counter {
                deal,
                seed,
                turn: view.turn,
                counterspell: spell.name.to_string(),
                source: cast_by
                    .get(&spell.id)
                    .cloned()
                    .unwrap_or_else(|| "?".to_owned()),
                target,
                kind,
                mana_value,
                whose,
                poor,
            });
        }
        if let PlayerAction::CastSpell { card } = decision.action {
            cast_by.insert(*card, decision.source.to_owned());
        }
        Ok(())
    })?;
    Ok(found)
}

/// Counts the seat's counterspells over every game of `run`; prints the
/// count as JSON, and writes it to `out` when given.
pub(super) fn count(run: &Path, threads: usize, out: Option<&Path>) -> anyhow::Result<()> {
    let index = BufReader::new(fs::File::open(run.join("games.jsonl"))?);
    let mut games = Vec::new();
    for line in index.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: Value = serde_json::from_str(&line)?;
        if let Some(deal) = row["i"].as_u64() {
            games.push((deal, row["seed"].as_u64()));
        }
    }
    let next = AtomicUsize::new(0);
    let counters = Mutex::new(Vec::new());
    let failed = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                while let Some(&(deal, seed)) = games.get(next.fetch_add(1, Ordering::Relaxed)) {
                    match game(run, deal, seed) {
                        Ok(found) => counters.lock().expect("counters lock").extend(found),
                        Err(error) => failed
                            .lock()
                            .expect("failed lock")
                            .push(json!({"deal": deal, "error": format!("{error:#}")})),
                    }
                }
            });
        }
    });
    let mut counters = counters.into_inner().expect("counters lock");
    counters.sort_by_key(|c| (c.deal, c.turn));
    let mut failed = failed.into_inner().expect("failed lock");
    failed.sort_by_key(|f| f["deal"].as_u64());
    let tally = |key: fn(&Counter) -> String| {
        let mut by: BTreeMap<String, u64> = BTreeMap::new();
        for c in &counters {
            *by.entry(key(c)).or_default() += 1;
        }
        by
    };
    let poor = counters.iter().filter(|c| c.poor).count();
    #[allow(clippy::cast_precision_loss, reason = "counts of one run")]
    let share = if counters.is_empty() {
        None
    } else {
        Some(poor as f64 / counters.len() as f64)
    };
    let report = json!({
        "run": run,
        "games": games.len(),
        "replayed": games.len() - failed.len(),
        "failed": failed,
        "counterspells": counters.len(),
        "poor": poor,
        "poor_share": share,
        "cheap_at_most": CHEAP,
        "by_whose": tally(|c| c.whose.clone()),
        "by_kind": tally(|c| c.kind.clone()),
        "by_mana_value": tally(|c| c.mana_value.map_or_else(|| "-".to_owned(), |mv| mv.to_string())),
        "by_source": tally(|c| c.source.clone()),
        "poor_cases": counters.iter().filter(|c| c.poor).collect::<Vec<_>>(),
        "not_judged": "whether the countered spell would have done anything",
    });
    let text = serde_json::to_string_pretty(&report)?;
    if let Some(out) = out {
        fs::write(out, &text)?;
    }
    println!("{text}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spell_is_named_by_its_first_type_as_a_reader_would() {
        assert_eq!(kind(TypeSet::CREATURE.union(TypeSet::ARTIFACT)), "creature");
        assert_eq!(kind(TypeSet::ARTIFACT), "artifact");
        assert_eq!(kind(TypeSet::INSTANT), "instant");
        assert_eq!(kind(TypeSet::EMPTY), "other");
    }

    #[test]
    fn whose_a_spell_is_reads_the_table_s_teams() {
        let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
        let teams = [Some(1), Some(2), Some(1)];
        assert_eq!(whose(p0, p0, &teams), "own");
        assert_eq!(whose(p0, p2, &teams), "teammate");
        assert_eq!(whose(p0, p1, &teams), "opponent");
        assert_eq!(whose(p0, p1, &[None, None]), "opponent");
    }
}
