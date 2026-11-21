//! Goad designation, resolution, expiration, and requirement lookup (CR 701.15).

use crate::{
    choice::Pending,
    object::{Rider, Status},
    resolve::Resolution,
    state::GameState,
    zone::Zone,
};
use baylee_cards_dsl::{Filter, Modifier, TargetSpec};
use baylee_core::{
    ids::{DamageSourceRef, ObjectId, PlayerId},
    types::TypeSet,
};
use smallvec::SmallVec;

/// One active goad designation on a battlefield permanent incarnation (CR 701.15a–d).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GoadEntry {
    /// Exact permanent incarnation that was goaded (CR 400.7).
    pub creature: DamageSourceRef,
    /// Controller of the spell or ability that goaded it (CR 701.15a–b).
    pub goader: PlayerId,
}

/// Designates `creature` as goaded by `goader` until `goader`'s next turn
/// (CR 701.15a–d). Idempotent per `(incarnation, goader)` (CR 701.15d).
pub fn goad_creature(state: &mut GameState, creature: ObjectId, goader: PlayerId) {
    if state.has_left(goader) {
        return;
    }
    let Some(obj) = state.object(creature) else {
        return;
    };
    if obj.zone != Zone::Battlefield
        || obj.status.contains(Status::PHASED_OUT)
        || !obj.characteristics().types.contains(TypeSet::CREATURE)
    {
        return;
    }
    let reference = DamageSourceRef {
        object: creature,
        version: obj.version,
    };
    // CR 701.15d: the same player goading the same creature again has no effect.
    if state
        .goaded
        .iter()
        .any(|entry| entry.creature == reference && entry.goader == goader)
    {
        return;
    }
    state.goaded.0.push(GoadEntry {
        creature: reference,
        goader,
    });
}

/// Resolves `Effect::Goad { target }` against live battlefield incarnations.
pub(crate) fn resolve_target(
    state: &mut GameState,
    res: &Resolution,
    target: TargetSpec,
) -> Option<Pending> {
    let goader = res.controller;
    match target {
        TargetSpec::ThisObject => {
            let expected_version = state.object(res.on_stack).and_then(|o| {
                o.riders.iter().find_map(|r| match r {
                    Rider::AbilitySourceVersion(v) => Some(*v),
                    _ => None,
                })
            });
            if state
                .object(res.source)
                .is_some_and(|o| expected_version.is_none_or(|v| o.version == v))
            {
                goad_creature(state, res.source, goader);
            }
        }
        TargetSpec::EventObject => {
            if let Some(id) = res.event_object {
                let expected_version = state.object(res.on_stack).and_then(|o| {
                    o.riders.iter().find_map(|r| match r {
                        Rider::EventObjectIdentity(v, _) => Some(*v),
                        _ => None,
                    })
                });
                if state
                    .object(id)
                    .is_some_and(|o| expected_version.is_none_or(|v| o.version == v))
                {
                    goad_creature(state, id, goader);
                }
            }
        }
        _ => {
            for &id in &res.targets {
                if target_still_current(state, res, id) {
                    goad_creature(state, id, goader);
                }
            }
        }
    }
    None
}

fn target_still_current(state: &GameState, res: &Resolution, id: ObjectId) -> bool {
    let Some(obj) = state.object(id) else {
        return false;
    };
    res.target_lki.as_ref().is_none_or(|all| {
        all.iter()
            .find(|lki| lki.id == id)
            .is_none_or(|lki| lki.version == obj.version)
    })
}

/// Resolves `Effect::GoadAll { filter, controlled_by }`.
pub(crate) fn resolve_all(
    state: &mut GameState,
    res: &Resolution,
    filter: &Filter,
    seats: Option<&[PlayerId]>,
) -> Option<Pending> {
    let context = res.rule_context();
    let targets: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            state.object(id).is_some_and(|obj| {
                seats.is_none_or(|s| s.contains(&obj.controller))
                    && crate::eval::matches_with_context(
                        filter,
                        state,
                        obj,
                        res.controller,
                        context,
                    )
            })
        })
        .collect();
    for id in targets {
        goad_creature(state, id, res.controller);
    }
    None
}

/// Distinct players currently goading `creature` (CR 701.15b–d), combining
/// resolution-applied designations and static `Modifier::Goaded` effects.
#[must_use]
pub fn goaders_of(state: &GameState, creature: ObjectId) -> SmallVec<[PlayerId; 2]> {
    let Some(obj) = state
        .object(creature)
        .filter(|o| o.zone == Zone::Battlefield && !o.status.contains(Status::PHASED_OUT))
    else {
        return SmallVec::new();
    };
    let reference = DamageSourceRef {
        object: creature,
        version: obj.version,
    };
    let mut out = SmallVec::new();
    for entry in &state.goaded.0 {
        if entry.creature == reference && !out.contains(&entry.goader) {
            out.push(entry.goader);
        }
    }
    for fx in state.effects.iter() {
        if fx.modifier == Modifier::Goaded
            && !out.contains(&fx.controller)
            && crate::effects::applies_to(state, fx, obj)
        {
            out.push(fx.controller);
        }
    }
    out
}

/// Expires goad designations created by `player` (CR 701.15a) and prunes any
/// entries whose target incarnation has left the battlefield (CR 400.7).
pub(crate) fn expire_for_turn(state: &mut GameState, player: PlayerId) {
    if state.goaded.is_empty() {
        return;
    }
    let mut kept = std::mem::take(&mut state.goaded.0);
    kept.retain(|entry| {
        entry.goader != player
            && state
                .object(entry.creature.object)
                .is_some_and(|o| o.zone == Zone::Battlefield && o.version == entry.creature.version)
    });
    state.goaded.0 = kept;
}

/// Expires goad designations across a normal turn transition from `after` to
/// `next`, including any eliminated seats passed over in turn order whose next
/// turn would have begun now (CR 800.4m).
pub(crate) fn expire_between(state: &mut GameState, after: PlayerId, next: PlayerId) {
    if state.goaded.is_empty() || state.players.is_empty() {
        return;
    }
    let count = state.players.len();
    let mut seat = (usize::from(after.get()) + 1) % count;
    for _ in 0..count {
        let player = PlayerId::new(seat as u8);
        expire_for_turn(state, player);
        if player == next {
            break;
        }
        seat = (seat + 1) % count;
    }
}
