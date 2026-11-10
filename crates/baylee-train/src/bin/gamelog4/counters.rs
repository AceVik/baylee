//! The evaluated seat's counterspells and what they countered, over every
//! game of a record directory: a regression watch for a net that reacts
//! more, that it does not start countering at random.
//!
//! A counterspell is a spell of the seat's on the stack that the tactical
//! layer reads as one (`reading::counters`, the reading `gamelog4 react` and
//! the deck scan share) and that names a spell or ability on the stack: one
//! cast for another of its modes, or a flash permanent that counters as it
//! enters, is not counted. It is read off the stack as the seat sees it
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
use baylee_engine::choice::{Pending, PlayerAction};
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
    /// Where and against what the seat cast it, when the cast was its own
    /// answer in the record.
    cast: Option<CastContext>,
}

/// The table as the seat cast a spell: whose turn and which step, the
/// spells already on the stack by whose, and the legal targets the seat
/// was asked to choose among (`None` when no target question came).
#[derive(Clone, Debug, Default, Serialize)]
struct CastContext {
    turn_of: String,
    step: String,
    own_spells_on_stack: u32,
    opponent_spells_on_stack: u32,
    targets_offered: Option<u32>,
    opponent_targets_offered: Option<u32>,
}

impl CastContext {
    /// The context of a cast answered at `view`.
    fn at(view: &PlayerView, side: &[PlayerId]) -> Self {
        let (mut own, mut opponent) = (0, 0);
        for spell in view
            .stack
            .iter()
            .filter(|o| matches!(o.stack_item, None | Some(StackItem::Spell)))
        {
            if whose(view.seat, spell.controller, side) == "opponent" {
                opponent += 1;
            } else {
                own += 1;
            }
        }
        Self {
            turn_of: whose(view.seat, view.active, side).to_owned(),
            step: format!("{:?}", view.step),
            own_spells_on_stack: own,
            opponent_spells_on_stack: opponent,
            targets_offered: None,
            opponent_targets_offered: None,
        }
    }

    /// Whether an opponent's spell stood to be aimed at instead: offered
    /// by the target question, or (no question) on the stack.
    fn opponent_alternative(&self) -> bool {
        match self.opponent_targets_offered {
            Some(offered) => offered > 0,
            None => self.opponent_spells_on_stack > 0,
        }
    }

    /// Whether the seat's own spell was the one legal target: the target
    /// question offered one option, or no question came and only one
    /// spell, the seat's, stood on the stack.
    fn own_only(&self) -> bool {
        match self.targets_offered {
            Some(offered) => offered == 1 && self.opponent_targets_offered == Some(0),
            None => self.opponent_spells_on_stack == 0 && self.own_spells_on_stack == 1,
        }
    }
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

/// Whose `controller` is, seen from `seat`, whose side (itself and its
/// teammates, `Game::side`) is `side`.
fn whose(seat: PlayerId, controller: PlayerId, side: &[PlayerId]) -> &'static str {
    if controller == seat {
        "own"
    } else if side.contains(&controller) {
        "teammate"
    } else {
        "opponent"
    }
}

/// Whether `spell` names a spell or ability on the stack other than itself.
fn names_the_stack(view: &PlayerView, spell: &PublicObject) -> bool {
    spell.targets.iter().any(|target| {
        matches!(target, TargetRef::Object(named)
            if named.object != spell.id && view.stack.iter().any(|item| item.id == named.object))
    })
}

/// What the counterspell `spell` names as its target, as `view` shows it:
/// name, kind, mana value, whose.
fn aimed(
    view: &PlayerView,
    spell: &PublicObject,
    side: &[PlayerId],
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
                        whose(view.seat, object.controller, side).to_owned(),
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
            whose(view.seat, *player, side).to_owned(),
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
    let side = game.side();
    let seat = usize::from(game.seat.get());
    // Who answered each cast, by the card's handle (a card keeps it from
    // hand to stack), and the counterspells already counted while they
    // stay on the stack.
    let mut cast_by: BTreeMap<ObjectId, String> = BTreeMap::new();
    let mut cast_at: BTreeMap<ObjectId, CastContext> = BTreeMap::new();
    let mut last_cast: Option<ObjectId> = None;
    let mut counted: BTreeSet<ObjectId> = BTreeSet::new();
    let mut found = Vec::new();
    game.replay(false, |event| {
        let Event::Decision(decision) = event else {
            return Ok(());
        };
        let view = &decision.views[seat];
        // The first target question after a cast is that cast's: its legal
        // options, and how many of them are an opponent's.
        if let Pending::ChooseTargets { options, .. } = decision.pending
            && let Some(card) = decision
                .asked
                .and_then(|asked| asked.announced_cast)
                .or(last_cast)
            && let Some(context) = cast_at.get_mut(&card)
            && context.targets_offered.is_none()
        {
            let opponent = options
                .iter()
                .filter(|id| {
                    view.stack.iter().any(|o| {
                        o.id == **id && whose(view.seat, o.controller, &side) == "opponent"
                    })
                })
                .count();
            context.targets_offered = u32::try_from(options.len()).ok();
            context.opponent_targets_offered = u32::try_from(opponent).ok();
        }
        counted.retain(|id| view.stack.iter().any(|o| o.id == *id));
        for spell in &view.stack {
            if spell.controller != view.seat
                || !matches!(spell.stack_item, None | Some(StackItem::Spell))
                || counted.contains(&spell.id)
                || !reading::counters(view, spell.id)
                || !names_the_stack(view, spell)
            {
                continue;
            }
            counted.insert(spell.id);
            let (target, kind, mana_value, whose) = aimed(view, spell, &side);
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
                cast: cast_at.get(&spell.id).cloned(),
            });
        }
        if let PlayerAction::CastSpell { card } = decision.action {
            cast_by.insert(*card, decision.source.to_owned());
            cast_at.insert(*card, CastContext::at(view, &side));
            last_cast = Some(*card);
        }
        Ok(())
    })?;
    Ok(found)
}

/// The counters aimed at the seat's own spells, by where they were cast:
/// whose turn and step, whether an opponent's spell was there to aim at
/// instead, and whether the own spell was the one legal target.
fn own_target(counters: &[Counter]) -> Value {
    let own: Vec<&Counter> = counters.iter().filter(|c| c.whose == "own").collect();
    let mut by_turn_step: BTreeMap<String, u64> = BTreeMap::new();
    let (mut known, mut alternative, mut only, mut asked) = (0, 0, 0, 0);
    for context in own.iter().filter_map(|c| c.cast.as_ref()) {
        known += 1;
        *by_turn_step
            .entry(format!("{} turn, {}", context.turn_of, context.step))
            .or_default() += 1;
        alternative += u64::from(context.opponent_alternative());
        only += u64::from(context.own_only());
        asked += u64::from(context.targets_offered.is_some());
    }
    json!({
        "count": own.len(),
        "cast_seen": known,
        "with_an_opponent_spell_to_aim_at": alternative,
        "own_spell_the_only_target": only,
        "target_question_asked": asked,
        "by_turn_and_step": by_turn_step,
    })
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
        "own_target": own_target(&counters),
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
    fn whose_a_spell_is_reads_the_seat_s_side() {
        let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
        let side = [p0, p2];
        assert_eq!(whose(p0, p0, &side), "own");
        assert_eq!(whose(p0, p2, &side), "teammate");
        assert_eq!(whose(p0, p1, &side), "opponent");
        assert_eq!(whose(p0, p1, &[p0]), "opponent");
    }

    #[test]
    fn a_cast_reads_its_target_question_before_the_stack() {
        let alone = CastContext {
            own_spells_on_stack: 1,
            ..CastContext::default()
        };
        assert!(alone.own_only() && !alone.opponent_alternative());
        let offered = CastContext {
            own_spells_on_stack: 1,
            opponent_spells_on_stack: 1,
            targets_offered: Some(1),
            opponent_targets_offered: Some(0),
            ..CastContext::default()
        };
        // An opponent's spell the counter cannot target is no alternative.
        assert!(offered.own_only() && !offered.opponent_alternative());
        let choice = CastContext {
            targets_offered: Some(2),
            opponent_targets_offered: Some(1),
            ..offered
        };
        assert!(!choice.own_only() && choice.opponent_alternative());
    }
}
