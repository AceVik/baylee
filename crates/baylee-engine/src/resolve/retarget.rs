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
//!
//! Each target group is kept distinct, so separate instances of "target"
//! may name the same object. Choices are staged until the complete final set
//! is legal, including swaps between targets within one group.
//!
//! [`Effect::ChangeTarget`]: baylee_cards_dsl::Effect::ChangeTarget
//! [`Effect::ChooseNewTargets`]: baylee_cards_dsl::Effect::ChooseNewTargets

use baylee_cards_dsl::{Filter, TargetReq, TargetSpec};
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};

use super::{AwaitingOp, Resolution};
use crate::choice::{Pending, TargetPrompt};
use crate::eval;
use crate::state::GameState;
use crate::zone::Zone;

pub use crate::choice::TargetRef as Aim;

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
    was: Vec<(u8, Aim)>,
    /// What each of `was` becomes, as far as it has been asked about.
    /// `None` stays.
    now: Vec<Option<Aim>>,
    /// A copy choosing its targets (CR 707.10c) and not a spell changing
    /// them: nothing it holds has been journalled as its target yet, so
    /// every target it ends with becomes one, kept or new.
    copy: bool,
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
    begin(state, res, spell, change_to, false)
}

/// "You may choose new targets for the copy" (CR 707.10c): the copy's
/// controller may leave any number of its targets unchanged, even illegal
/// ones, and change the rest to legal ones, which is CR 115.7d's question
/// asked of `copy`. Once it is answered, every target the copy holds is
/// journalled as newly targeted: the copy is a new object, and "becomes the
/// target" sees it arrive with all of them.
pub(super) fn start_copy(
    state: &mut GameState,
    res: &mut Resolution,
    copy: ObjectId,
) -> Option<Pending> {
    if state.object(copy).is_none_or(|o| o.target_req.is_none()) {
        record_new_targets(state, copy, &[], &[]);
        return None;
    }
    begin(state, res, copy, None, true)
}

fn begin(
    state: &mut GameState,
    res: &mut Resolution,
    spell: ObjectId,
    change_to: Option<&'static Filter>,
    copy: bool,
) -> Option<Pending> {
    state.capture_source_references();
    let obj = state.object(spell).filter(|o| o.zone == Zone::Stack)?;
    let req = obj.target_req?;
    let was: Vec<(u8, Aim)> = obj
        .targets
        .iter()
        .enumerate()
        .map(|(index, _)| {
            (
                0,
                Aim::Object(
                    state
                        .recorded_target_reference(
                            spell,
                            false,
                            u32::try_from(index).expect("target slot"),
                        )
                        .expect("announced target"),
                ),
            )
        })
        .chain(
            eval::targeted_players(obj, &req.spec)
                .iter()
                .map(|p| (0, Aim::Player(p))),
        )
        .chain(obj.second_targets().iter().enumerate().map(|(index, _)| {
            (
                1,
                Aim::Object(
                    state
                        .recorded_target_reference(
                            spell,
                            true,
                            u32::try_from(index).expect("target slot"),
                        )
                        .expect("announced second target"),
                ),
            )
        }))
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
            copy,
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
        .map(|&id| Aim::Object(state.source_identity(id).expect("offered target exists")))
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
            let slot = retarget.now.len();
            let keep = retarget.was[slot].1;
            let min =
                u32::from(retarget.change_to.is_some() || !can_finish(state, &retarget, keep));
            let reason = TargetPrompt::Retarget {
                current: keep,
                index: u32::try_from(slot).unwrap_or(u32::MAX),
                of: u32::try_from(retarget.was.len()).unwrap_or(u32::MAX),
            };
            res.awaiting = Some(AwaitingOp::NewTargets(Box::new(retarget)));
            return Some(Pending::ChooseTargets {
                player: res.controller,
                options,
                player_options,
                min,
                max: 1,
                reason,
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
    let req = requirement(obj, retarget.was[slot].0)?;
    let (mut objects, mut players) = eval::stack_target_options(state, obj, &req.spec);
    objects.retain(|id| {
        let Some(reference) = state.source_identity(*id) else {
            return false;
        };
        Aim::Object(reference) != retarget.was[slot].1
            && can_finish(state, retarget, Aim::Object(reference))
    });
    players.retain(|p| {
        Aim::Player(*p) != retarget.was[slot].1 && can_finish(state, retarget, Aim::Player(*p))
    });
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
    state.capture_source_references();
    let references = retarget_references(retarget);
    let previous: Vec<_> = if retarget.copy {
        Vec::new()
    } else {
        retarget
            .was
            .iter()
            .filter_map(|(_, target)| match target {
                Aim::Object(reference) => Some(*reference),
                Aim::Player(_) => None,
            })
            .collect()
    };
    let previous_players = state
        .object(retarget.spell)
        .filter(|_| !retarget.copy)
        .map_or_else(Vec::new, targeted_players);
    let Some(obj) = state.object_mut(retarget.spell) else {
        return;
    };
    let first_req = obj.target_req;
    let second_req = obj.second_target_req();
    obj.targets.clear();
    obj.target_players = SeatSet::new();
    if first_req
        .is_some_and(|req| matches!(req.spec, TargetSpec::AnyPlayer | TargetSpec::AnyOpponent))
    {
        obj.chosen_player = None;
    }
    let mut second = smallvec::SmallVec::new();
    for ((group, was), now) in retarget.was.iter().zip(&retarget.now) {
        let aim = now.unwrap_or(*was);
        match (*group, aim) {
            (0, Aim::Object(reference)) => obj.targets.push(reference.object),
            (0, Aim::Player(player)) => {
                if first_req.is_some_and(|req| {
                    matches!(req.spec, TargetSpec::AnyPlayer | TargetSpec::AnyOpponent)
                }) {
                    obj.chosen_player = Some(player);
                } else {
                    obj.target_players.insert(player);
                }
            }
            (_, Aim::Object(reference)) => second.push(reference.object),
            (_, Aim::Player(_)) => unreachable!("second target groups contain objects"),
        }
    }
    obj.set_second(second, second_req);
    let mut targets = crate::sources::TargetReferences::default();
    for (second, _, reference) in references {
        if second {
            targets.second.push(reference);
        } else {
            targets.first.push(reference);
        }
    }
    state.bind_target_references(retarget.spell, &targets);

    if let Some((_, shares)) = state
        .divided
        .iter_mut()
        .find(|(id, _)| *id == retarget.spell)
    {
        for (target, _) in shares {
            if let Some((slot, _)) = retarget
                .was
                .iter()
                .enumerate()
                .find(|(_, (group, aim))| *group == 0 && *aim == Aim::Object(*target))
                && let Some(Aim::Object(new)) = retarget.now[slot]
            {
                *target = new;
            }
        }
    }
    state.capture_source_references();
    record_new_targets(state, retarget.spell, &previous, &previous_players);
}

/// Object slots compact when a target becomes a player. Carry the retained
/// incarnation with its target rather than re-reading its newer zone object.
fn retarget_references(retarget: &Retarget) -> Vec<(bool, u32, baylee_core::ids::DamageSourceRef)> {
    let mut new_indices = [0_u32; 2];
    let mut references = Vec::new();
    for ((group, was), now) in retarget.was.iter().zip(&retarget.now) {
        let group = usize::from(*group != 0);
        if let Aim::Object(reference) = now.unwrap_or(*was) {
            references.push((group != 0, new_indices[group], reference));
            new_indices[group] += 1;
        }
    }
    references
}

/// The players a spell or ability on the stack targets: those "any target"
/// put beside its objects, and the one a "target player" or "target
/// opponent" requirement chose.
fn targeted_players(obj: &crate::object::GameObject) -> Vec<PlayerId> {
    let mut players: Vec<_> = obj.target_players.iter().chain(obj.chosen_player).collect();
    players.sort_unstable();
    players.dedup();
    players
}

fn requirement(obj: &crate::object::GameObject, group: u8) -> Option<TargetReq> {
    if group == 0 {
        obj.target_req
    } else {
        obj.second_target_req()
    }
}

/// Whether this staged choice leaves a distinct legal completion. The legal
/// alternatives are shared within one target group; each remaining slot can
/// also retain its original target even if that target is now illegal.
fn can_finish(state: &GameState, retarget: &Retarget, candidate: Aim) -> bool {
    let slot = retarget.now.len();
    let group = retarget.was[slot].0;
    let Some(obj) = state.object(retarget.spell) else {
        return false;
    };
    let Some(req) = requirement(obj, group) else {
        return false;
    };
    let used: Vec<_> = retarget.was[..slot]
        .iter()
        .zip(&retarget.now)
        .filter(|((g, _), _)| *g == group)
        .map(|((_, old), new)| new.unwrap_or(*old))
        .collect();
    if used.contains(&candidate) {
        return false;
    }
    let (objects, players) = eval::stack_target_options(state, obj, &req.spec);
    let mut available: Vec<Aim> = objects
        .into_iter()
        .filter_map(|id| state.source_identity(id).map(Aim::Object))
        .chain(players.into_iter().map(Aim::Player))
        .filter(|aim| *aim != candidate && !used.contains(aim))
        .collect();
    let remaining: Vec<_> = retarget.was[slot + 1..]
        .iter()
        .filter(|(g, _)| *g == group)
        .collect();
    for (_, original) in &remaining {
        if *original != candidate && !used.contains(original) && !available.contains(original) {
            available.push(*original);
        }
    }
    available.len() >= remaining.len()
}

/// Journal each newly acquired target once, objects and players. Copies pass
/// empty previous sets after choosing their final targets (CR 707.10c);
/// retargeting passes both original target groups, so a retained target cannot
/// trigger a second ward.
pub(super) fn record_new_targets(
    state: &mut GameState,
    spell: ObjectId,
    previous: &[baylee_core::ids::DamageSourceRef],
    previous_players: &[PlayerId],
) {
    let Some(obj) = state.object(spell) else {
        return;
    };
    let controller = obj.controller;
    let players: Vec<_> = targeted_players(obj)
        .into_iter()
        .filter(|p| !previous_players.contains(p))
        .collect();
    let mut targets = Vec::new();
    for second in [false, true] {
        let ids = if second {
            obj.second_targets()
        } else {
            &obj.targets
        };
        for index in 0..ids.len() {
            if let Some(reference) = state.recorded_target_reference(
                spell,
                second,
                u32::try_from(index).expect("target slot"),
            ) && !previous.contains(&reference)
                && state.source_identity(reference.object) == Some(reference)
            {
                targets.push(reference);
            }
        }
    }
    targets.sort_unstable();
    targets.dedup();
    for target in targets {
        state.journal.record(crate::event::GameEvent::BecameTarget {
            object: spell,
            target: target.object,
            controller,
        });
    }
    for player in players {
        state
            .journal
            .record(crate::event::GameEvent::PlayerBecameTarget {
                object: spell,
                player,
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
    fn replacing_an_object_with_a_player_keeps_later_historical_target_incarnations() {
        let mut state =
            GameState::from_preset(&preset(412, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let caster = PlayerId::new(0);
        let name = state.names.intern("retarget source");
        let spell = state.create_bare(caster, ObjectKind::Spell, name, ZoneLocation::Stack);
        let first = state.create_bare(
            caster,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let kept = state.create_bare(
            caster,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let kept_ref = state.source_identity(kept).unwrap();
        let first_ref = state.source_identity(first).unwrap();
        state
            .object_mut(spell)
            .unwrap()
            .targets
            .extend([first, kept]);
        state
            .move_object(
                kept,
                ZoneLocation::Hand(caster),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                kept,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        write(
            &mut state,
            &Retarget {
                spell,
                change_to: None,
                was: vec![(0, Aim::Object(first_ref)), (0, Aim::Object(kept_ref))],
                now: vec![Some(Aim::Player(caster)), None],
                copy: false,
            },
        );
        assert_eq!(state.object(spell).unwrap().targets.as_slice(), &[kept]);
        assert_eq!(state.recorded_stack_target(spell, 0), Some(kept_ref));
        assert_eq!(state.recorded_stack_target(spell, 1), None);
        assert_ne!(state.source_identity(kept), Some(kept_ref));
    }

    #[test]
    fn target_acquisition_ignores_retained_targets_and_deduplicates_both_slots() {
        let mut state =
            GameState::from_preset(&preset(408, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let player = PlayerId::new(1);
        let name = state.names.intern("copy");
        let spell = state.create_bare(player, ObjectKind::Spell, name, ZoneLocation::Stack);
        let first = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let second = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let first_ref = state.source_identity(first).unwrap();
        let second_ref = state.source_identity(second).unwrap();
        let obj = state.object_mut(spell).unwrap();
        obj.targets.extend([first, second]);
        obj.set_second(smallvec::smallvec![second], None);
        state.capture_source_references();
        let start = state.journal.len();
        record_new_targets(&mut state, spell, &[second_ref, first_ref], &[]);
        assert_eq!(
            state.journal.len(),
            start,
            "keeping or reordering targets is not acquiring them"
        );
        record_new_targets(&mut state, spell, &[first_ref], &[]);
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
        record_new_targets(&mut state, spell, &[], &[]);
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
    /// A player a copy or a retarget newly aims at is journalled too, once,
    /// whichever of the two fields holds it — and a player the spell already
    /// held is not (Leovold reads this for "you … become the target").
    #[test]
    fn a_newly_targeted_player_is_journalled_and_a_kept_one_is_not() {
        let mut state =
            GameState::from_preset(&preset(409, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let caster = PlayerId::new(1);
        let (kept, new) = (PlayerId::new(0), PlayerId::new(1));
        let name = state.names.intern("copy");
        let spell = state.create_bare(caster, ObjectKind::Spell, name, ZoneLocation::Stack);
        let obj = state.object_mut(spell).unwrap();
        obj.target_players.insert(kept);
        obj.chosen_player = Some(new);
        let start = state.journal.len();
        record_new_targets(&mut state, spell, &[], &[kept]);
        assert_eq!(
            state.journal.entries()[start..]
                .iter()
                .map(|e| &e.event)
                .collect::<Vec<_>>(),
            vec![&GameEvent::PlayerBecameTarget {
                object: spell,
                player: new,
                controller: caster
            }]
        );
    }
    #[test]
    fn staged_swaps_preserve_targets_and_require_a_legal_completion() {
        let mut state =
            GameState::from_preset(&preset(410, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let player = PlayerId::new(0);
        let name = state.names.intern("retarget test");
        let spell = state.create_bare(player, ObjectKind::Spell, name, ZoneLocation::Stack);
        let a = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let b = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        for id in [a, b] {
            state.object_mut(id).unwrap().base_mut().types = baylee_core::types::TypeSet::CREATURE;
        }
        let obj = state.object_mut(spell).unwrap();
        obj.target_req = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)));
        obj.targets.extend([a, b]);
        let a_ref = state.source_identity(a).unwrap();
        let b_ref = state.source_identity(b).unwrap();
        let mut choice = Retarget {
            spell,
            change_to: None,
            was: vec![(0, Aim::Object(a_ref)), (0, Aim::Object(b_ref))],
            now: vec![],
            copy: true,
        };
        assert!(
            can_finish(&state, &choice, Aim::Object(b_ref)),
            "the complete swap is legal"
        );
        choice.now.push(Some(Aim::Object(b_ref)));
        assert!(
            !can_finish(&state, &choice, Aim::Object(b_ref)),
            "the second target cannot retain a duplicate"
        );
        assert!(can_finish(&state, &choice, Aim::Object(a_ref)));
        choice.now.push(Some(Aim::Object(a_ref)));
        state.divided.push((spell, vec![(a_ref, 3), (b_ref, 1)]));
        write(&mut state, &choice);
        assert_eq!(
            state.object(spell).unwrap().targets.as_slice(),
            &[b, a],
            "staged writes cannot overwrite both occurrences"
        );
        assert_eq!(
            state.divided[0].1,
            vec![(b_ref, 3), (a_ref, 1)],
            "each fixed damage share moves with its slot"
        );
        choice.now.clear();
        state.object_mut(a).unwrap().base_mut().types = baylee_core::types::TypeSet::ARTIFACT;
        assert!(
            !can_finish(&state, &choice, Aim::Object(b_ref)),
            "a swap may not make a newly chosen target illegal"
        );
        assert!(
            can_finish(&state, &choice, Aim::Object(a_ref)),
            "the existing illegal target can remain unchanged"
        );
    }
    #[test]
    fn target_events_distinguish_kept_old_incarnation_from_the_returned_creature() {
        let mut state =
            GameState::from_preset(&preset(503, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let player = PlayerId::new(0);
        let name = state.names.intern("target event probe");
        let target = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let original = state.source_identity(target).unwrap();
        let spell = state.create_bare(player, ObjectKind::Spell, name, ZoneLocation::Stack);
        state.object_mut(spell).unwrap().targets.push(target);
        state
            .move_object(
                target,
                ZoneLocation::Hand(player),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                target,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        let current = state.source_identity(target).unwrap();
        let start = state.journal.len();
        write(
            &mut state,
            &Retarget {
                spell,
                change_to: None,
                was: vec![(0, Aim::Object(original))],
                now: vec![None],
                copy: true,
            },
        );
        assert_eq!(
            state.journal.len(),
            start,
            "copying an illegal old target cannot trigger the returned creature's ward"
        );
        write(
            &mut state,
            &Retarget {
                spell,
                change_to: None,
                was: vec![(0, Aim::Object(original))],
                now: vec![Some(Aim::Object(current))],
                copy: false,
            },
        );
        assert_eq!(state.journal.entries()[start..].iter().filter(|entry| matches!(entry.event, GameEvent::BecameTarget { target: id, .. } if id == target)).count(), 1);
        assert_eq!(
            state.recorded_target_reference(spell, false, 0),
            Some(current)
        );
        write(
            &mut state,
            &Retarget {
                spell,
                change_to: None,
                was: vec![(0, Aim::Object(current))],
                now: vec![Some(Aim::Player(player))],
                copy: false,
            },
        );
        assert_eq!(state.recorded_target_reference(spell, false, 0), None);
        assert!(
            !state.source_referenced_by(current).contains(&spell),
            "compaction drops obsolete object references"
        );
    }
}
