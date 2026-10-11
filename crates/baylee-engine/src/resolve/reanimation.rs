//! Returning cards to the battlefield, including double-faced returns.
use super::zones::moving_objects;
#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;
use crate::{state::CardLookup, zone::Zone};

pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::GraveyardToBattlefield {
            target,
            owner_control,
            counters,
        } => {
            for id in moving_objects(state, res, target) {
                if matches!(target, TargetSpec::EventObject)
                    && subjects::event(state, res).is_some_and(|r| !subjects::is_current(state, r))
                {
                    continue;
                }
                return_card(state, id, you, owner_control, counters);
            }
            None
        }
        Effect::Assimilate { target } => {
            assimilate(state, res, target);
            None
        }
        Effect::ReturnSourceFromGraveyard {
            face,
            tapped,
            owner_control,
        } => {
            if let Some(source) = subjects::moving_source(state, res)
                && state
                    .object(source.object)
                    .is_some_and(|o| o.zone == Zone::Graveyard)
            {
                arrive(state, source.object, you, owner_control, face, tapped);
            }
            None
        }
        Effect::ExileSelfReturnAsFace {
            face,
            owner_control,
        } => {
            let source =
                subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))?;
            let owner = state.object(source.object).expect("current source").owner;
            let _ = state.move_object(
                source.object,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            if state
                .object(source.object)
                .is_some_and(|o| o.zone == Zone::Exile)
            {
                arrive(state, source.object, you, owner_control, Some(face), false);
            }
            None
        }
        _ => unreachable!("only return operations are dispatched here"),
    }
}

/// CR 611.2e applies assimilation's characteristic changes during entry.
fn assimilate(state: &mut GameState, res: &Resolution, target: TargetSpec) {
    use baylee_cards_dsl::{CounterKind, Duration, Modifier};
    use baylee_core::{generated::subtypes, ids::EffectId, types::TypeSet};
    if state.has_left(res.controller) {
        return;
    }
    for id in moving_objects(state, res, target) {
        let Some(object) = state.object(id).filter(|o| o.zone == Zone::Graveyard) else {
            continue;
        };
        let version = object.version.wrapping_add(1);
        let mut created = smallvec::SmallVec::<[EffectId; 2]>::new();
        for modifier in [
            Modifier::AddType(TypeSet::ARTIFACT.union(TypeSet::CREATURE)),
            Modifier::ReplaceCreatureTypes(subtypes::creature::BORG),
        ] {
            let timestamp = state.next_timestamp();
            created.push(state.effects.register(crate::effects::ContinuousEffect {
                id: EffectId::new(0),
                source: None,
                controller: res.controller,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp,
                duration: Duration::Indefinitely,
                filter: crate::effects::EffectFilter::ObjectIs(id, version),
                modifier,
            }));
        }
        return_card(
            state,
            id,
            res.controller,
            false,
            Some((CounterKind::P1P1, 1)),
        );
        if !state
            .object(id)
            .is_some_and(|o| o.zone == Zone::Battlefield && o.version == version)
        {
            state
                .effects
                .remove_where(|effect| created.contains(&effect.id));
        }
    }
}

/// The shared arrival door for ordinary and characteristic-changing returns.
fn return_card(
    state: &mut GameState,
    id: ObjectId,
    controller: PlayerId,
    owner_control: bool,
    counters: Option<(baylee_cards_dsl::CounterKind, u16)>,
) {
    if !owner_control && state.has_left(controller) {
        return;
    }
    let Some(object) = state
        .object_mut(id)
        .filter(|o| o.zone == Zone::Graveyard && o.is_card())
    else {
        return;
    };
    object.kind = ObjectKind::Permanent;
    object.set_controller(if owner_control {
        object.owner
    } else {
        controller
    });
    let _ = state.move_object(
        id,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
    // The zone change clears counters. Install the entry counters before entry
    // abilities are collected, through the replacement-aware placement door.
    if let Some((kind, n)) = counters
        && n > 0
        && state
            .object(id)
            .is_some_and(|o| o.zone == Zone::Battlefield)
    {
        crate::replacement::put_counters(state, id, kind, n);
    }
}

/// Choose the entering face before any entry replacement or trigger can see it.
/// Entering transformed is not the transform action (CR 712.14a, 701.27a).
fn arrive(
    state: &mut GameState,
    id: ObjectId,
    you: PlayerId,
    owner_control: bool,
    face: Option<u8>,
    tapped: bool,
) {
    if !owner_control && state.has_left(you) {
        return;
    }
    let Some(object) = state.object(id) else {
        return;
    };
    if !object.is_card() {
        return;
    } // A token that left cannot return (CR 111.8).
    if let Some(face) = face {
        let Some(def) = object.card.and_then(|card| state.bases.card(card.index)) else {
            return;
        };
        let Some(entering) = def.faces.get(usize::from(face)) else {
            return;
        };
        if !entering.types.is_permanent()
            || (face != 0
                && (entering.adventure
                    || def.faces[0]
                        .subtypes
                        .contains(&baylee_core::generated::subtypes::enchantment::ROOM)
                    || (entering.castable_from_hand && !def.faces[0].types.is_permanent())))
        {
            return;
        }
        state.turn_over(id, def, usize::from(face));
    }
    if state
        .object(id)
        .is_none_or(|o| !o.characteristics().types.is_permanent())
    {
        return;
    }
    if let Some(object) = state.object_mut(id) {
        object.kind = ObjectKind::Permanent;
        object.set_controller(if owner_control { object.owner } else { you });
    }
    state.set_tapped(id, tapped);
    let _ = state.move_object(
        id,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
}
