//! Changing the targets of a spell or ability on the stack (CR 115.7).
//!
//! Two sentences, two rules:
//!
//! - "Change the target of target spell" ([`Effect::ChangeTarget`], CR
//!   115.7a): each target may move only to **another** legal target, and it
//!   is all of them or none. With no other legal target, the original stays,
//!   even if it is illegal by then.
//! - "You may choose new targets for target spell or ability"
//!   ([`Effect::ChooseNewTargets`], CR 115.7d): any number of the targets
//!   may stay, and a new one must be legal.
//!
//! "Legal" is the spell's own requirement, asked of the spell's own
//! controller: [`eval::stack_target_options`], the enumeration CR 608.2b's
//! re-check asks too. So a Path to Exile is never turned onto a Plains, and
//! a Lightning Bolt aimed at a face may be turned onto another player.
//!
//! One target is asked about at a time, in the order the spell holds them:
//! objects first, then players. A question offers what that target may
//! become. Its current target is never offered: under 115.7a it is not
//! "another" target, and under 115.7d staying is answered by naming nothing
//! (`min: 0`). Nothing is written until every target has been asked about.
//!
//! What this does not do:
//! - **115.7a changes a spell with one target only.** Every card that says
//!   "change the target" prints "with a single target" (CR 115.9a), which
//!   `Filter::WithSingleTarget` reads as the card targets. A spell that holds
//!   two anyway (a filter written without it) is left alone and nothing is
//!   asked: none is 115.7a's answer too.
//! - **A swap between two targets is not offered.** Another target's
//!   current object or player is kept out of each question, because under
//!   115.7d it may stay, and two instances of one target would break CR
//!   115.3. CR 115.7e would allow a swap, since only the final set is
//!   judged.
//! - **An ability's targets stay** when it carries no `target_req`: its
//!   requirement is in its definition, which the resolver cannot read
//!   without a card lookup (#249).
//! - **Only the first instance of "target"** is asked about. A second one
//!   (a fight's other creature) keeps its target, which is a legal answer
//!   under 115.7d and a gap under 115.7a.
//!
//! [`Effect::ChangeTarget`]: baylee_cards_dsl::Effect::ChangeTarget
//! [`Effect::ChooseNewTargets`]: baylee_cards_dsl::Effect::ChooseNewTargets

use baylee_cards_dsl::Filter;
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};

use super::{AwaitingOp, Resolution};
use crate::choice::{Pending, TargetPrompt};
use crate::eval;
use crate::state::GameState;
use crate::zone::Zone;

/// One target a spell holds: an object or a player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aim {
    /// An object target.
    Object(ObjectId),
    /// A player target.
    Player(PlayerId),
}

/// A change of targets waiting on its next answer.
#[derive(Clone, Debug)]
pub struct Retarget {
    /// The spell or ability whose targets change.
    spell: ObjectId,
    /// For [`Effect::ChangeTarget`](baylee_cards_dsl::Effect::ChangeTarget):
    /// what a new target must match beside being legal. `None` is
    /// [`Effect::ChooseNewTargets`](baylee_cards_dsl::Effect::ChooseNewTargets).
    change_to: Option<&'static Filter>,
    /// What the spell targeted when the effect began, in order.
    was: Vec<Aim>,
    /// What each of `was` becomes, as far as it has been asked about.
    /// `None` stays.
    now: Vec<Option<Aim>>,
}

/// Starts a change of the first target's targets. `change_to` is `Some` for
/// "change the target" (CR 115.7a) and `None` for "choose new targets" (CR
/// 115.7d).
pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    change_to: Option<&'static Filter>,
) -> Option<Pending> {
    let &spell = res.targets.first()?;
    let obj = state.object(spell).filter(|o| o.zone == Zone::Stack)?;
    let req = obj.target_req?;
    let was: Vec<Aim> = obj
        .targets
        .iter()
        .map(|&id| Aim::Object(id))
        .chain(
            eval::targeted_players(obj, &req.spec)
                .iter()
                .map(Aim::Player),
        )
        .collect();
    if change_to.is_some() && was.len() != 1 {
        return None;
    }
    ask(
        state,
        res,
        Retarget {
            spell,
            change_to,
            was,
            now: Vec::new(),
        },
    )
}

/// Records the answer about one target and asks about the next.
pub(super) fn answer(
    state: &mut GameState,
    res: &mut Resolution,
    mut retarget: Retarget,
    objects: &[ObjectId],
    players: &[PlayerId],
) -> Option<Pending> {
    let pick = objects
        .first()
        .map(|&id| Aim::Object(id))
        .or_else(|| players.first().map(|&p| Aim::Player(p)));
    retarget.now.push(pick);
    ask(state, res, retarget)
}

/// Asks about the next target, skipping one under 115.7d that has nothing
/// to become. Writes the change once every target has been asked about.
fn ask(state: &mut GameState, res: &mut Resolution, mut retarget: Retarget) -> Option<Pending> {
    while retarget.now.len() < retarget.was.len() {
        let (options, player_options) = options(state, res, &retarget)?;
        if !options.is_empty() || !player_options.is_empty() {
            let min = u8::from(retarget.change_to.is_some());
            res.awaiting = Some(AwaitingOp::NewTargets(Box::new(retarget)));
            return Some(Pending::ChooseTargets {
                player: res.controller,
                options,
                player_options,
                min,
                max: 1,
                reason: TargetPrompt::Targets,
            });
        }
        if retarget.change_to.is_some() {
            // CR 115.7a: with no other legal target, the original stays,
            // even if it is illegal by then.
            return None;
        }
        retarget.now.push(None);
    }
    write(state, &retarget);
    None
}

/// What the target being asked about may become. `None` when the spell has
/// left the stack, which ends the change with nothing written.
fn options(
    state: &GameState,
    res: &Resolution,
    retarget: &Retarget,
) -> Option<(Vec<ObjectId>, Vec<PlayerId>)> {
    let slot = retarget.now.len();
    let obj = state
        .object(retarget.spell)
        .filter(|o| o.zone == Zone::Stack)?;
    let req = obj.target_req?;
    let (mut objects, mut players) = eval::stack_target_options(state, obj, &req.spec);
    // Its own target, what an earlier one became, and what a later one
    // still holds.
    let taken: Vec<Aim> = retarget
        .was
        .iter()
        .enumerate()
        .map(|(i, was)| retarget.now.get(i).copied().flatten().unwrap_or(*was))
        .enumerate()
        .filter(|(i, _)| *i != slot)
        .map(|(_, aim)| aim)
        .chain(std::iter::once(retarget.was[slot]))
        .collect();
    objects.retain(|id| !taken.contains(&Aim::Object(*id)));
    players.retain(|p| !taken.contains(&Aim::Player(*p)));
    if let Some(to) = retarget.change_to {
        objects.retain(|id| {
            state
                .object(*id)
                .is_some_and(|o| eval::matches(to, state, o, res.controller, res.source))
        });
        // A filter reads objects; only "any" lets a player through.
        if !matches!(to, Filter::Any) {
            players.clear();
        }
    }
    Some((objects, players))
}

/// Writes the finished change onto the spell, each target where the spell
/// kept it: an object in `targets`, in its place, and a player in
/// `target_players` or `chosen_player`, whichever held it.
fn write(state: &mut GameState, retarget: &Retarget) {
    let previous = state.object(retarget.spell).map_or_else(Vec::new, |obj| {
        obj.targets
            .iter()
            .chain(obj.second_targets())
            .copied()
            .collect()
    });
    let Some(obj) = state.object_mut(retarget.spell) else {
        return;
    };
    for (was, now) in retarget.was.iter().zip(&retarget.now) {
        let Some(now) = *now else { continue };
        match (*was, now) {
            (Aim::Object(old), Aim::Object(new)) => {
                for id in &mut obj.targets {
                    if *id == old {
                        *id = new;
                    }
                }
            }
            (Aim::Object(old), Aim::Player(new)) => {
                obj.targets.retain(|id| *id != old);
                obj.target_players.insert(new);
            }
            (Aim::Player(old), now) => {
                let new = match now {
                    Aim::Player(new) => Some(new),
                    Aim::Object(new) => {
                        obj.targets.push(new);
                        None
                    }
                };
                if obj.target_players.contains(old) {
                    obj.target_players = without(obj.target_players, old);
                    if let Some(new) = new {
                        obj.target_players.insert(new);
                    }
                }
                if obj.chosen_player == Some(old) {
                    obj.chosen_player = new;
                }
            }
        }
    }
    record_new_targets(state, retarget.spell, &previous);
}

/// `set` without `player`.
fn without(set: SeatSet, player: PlayerId) -> SeatSet {
    let mut out = SeatSet::new();
    for p in set.iter().filter(|p| *p != player) {
        out.insert(p);
    }
    out
}

/// Journal each newly acquired object target once. Copies pass an empty previous
/// set after choosing their final targets (CR 707.10c); retargeting passes both
/// original target groups, so a retained target cannot trigger a second ward.
pub(super) fn record_new_targets(state: &mut GameState, spell: ObjectId, previous: &[ObjectId]) {
    let Some(obj) = state.object(spell) else {
        return;
    };
    let controller = obj.controller;
    let mut targets: Vec<_> = obj
        .targets
        .iter()
        .chain(obj.second_targets())
        .copied()
        .filter(|id| !previous.contains(id))
        .collect();
    targets.sort_unstable();
    targets.dedup();
    for target in targets {
        state.journal.record(crate::event::GameEvent::BecameTarget {
            object: spell,
            target,
            controller,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::event::GameEvent;
    use crate::object::ObjectKind;
    use crate::zone::ZoneLocation;

    #[test]
    fn target_acquisition_ignores_retained_targets_and_deduplicates_both_slots() {
        let mut state =
            GameState::from_preset(&preset(408, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let player = PlayerId::new(1);
        let name = state.names.intern("copy");
        let spell = state.create_bare(player, ObjectKind::Spell, name, ZoneLocation::Stack);
        let first = ObjectId::new(100, 0);
        let second = ObjectId::new(101, 0);
        let obj = state.object_mut(spell).unwrap();
        obj.targets.extend([first, second]);
        obj.set_second(smallvec::smallvec![second], None);
        let start = state.journal.len();
        record_new_targets(&mut state, spell, &[second, first]);
        assert_eq!(
            state.journal.len(),
            start,
            "keeping or reordering targets is not acquiring them"
        );
        record_new_targets(&mut state, spell, &[first]);
        assert_eq!(
            state.journal.entries()[start..]
                .iter()
                .map(|e| &e.event)
                .collect::<Vec<_>>(),
            vec![&GameEvent::BecameTarget {
                object: spell,
                target: second,
                controller: player
            }]
        );
        let start = state.journal.len();
        record_new_targets(&mut state, spell, &[]);
        let targets: Vec<_> = state.journal.entries()[start..]
            .iter()
            .map(|e| match e.event {
                GameEvent::BecameTarget { target, .. } => target,
                _ => panic!("only targeting events"),
            })
            .collect();
        assert_eq!(
            targets,
            [first, second],
            "a copy acquires both targets, each just once"
        );
    }
}
