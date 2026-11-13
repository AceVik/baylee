//! Returning cards to the battlefield, including double-faced returns.
use super::zones::moving_objects;
#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;
use crate::{state::CardLookup, zone::Zone};

#[allow(clippy::too_many_lines)] // one dispatch arm per return family; shared work stays in helpers
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::GraveyardToBattlefieldLinked { target } => {
            for id in moving_objects(state, res, target) {
                let Some(before) = state.source_identity(id).filter(|reference| {
                    state
                        .object(reference.object)
                        .is_some_and(|object| object.zone == Zone::Graveyard && object.is_card())
                }) else {
                    continue;
                };
                return_card(state, id, you, false, None);
                super::linked_reanimation::queue(
                    state,
                    res,
                    baylee_core::ids::DamageSourceRef {
                        object: id,
                        version: before.version.wrapping_add(1),
                    },
                );
            }
            None
        }
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
        Effect::GraveyardToBattlefieldModified {
            target,
            owner_control,
            tapped,
            counters,
            modifiers,
        } => {
            return_modified(
                state,
                res,
                target,
                owner_control,
                tapped,
                counters,
                modifiers,
            );
            None
        }
        Effect::Assimilate { target } => {
            use baylee_cards_dsl::{CounterKind, Modifier};
            use baylee_core::{generated::subtypes, types::TypeSet};
            const MODIFIERS: &[Modifier] = &[
                Modifier::AddType(TypeSet::ARTIFACT.union(TypeSet::CREATURE)),
                Modifier::ReplaceCreatureTypes(subtypes::creature::BORG),
            ];
            return_modified(
                state,
                res,
                target,
                false,
                false,
                Some((CounterKind::P1P1, 1)),
                MODIFIERS,
            );
            None
        }
        Effect::ReturnSourceFromGraveyard {
            face,
            tapped,
            owner_control,
            counters,
        } => {
            if let Some(source) = subjects::moving_source(state, res)
                && state
                    .object(source.object)
                    .is_some_and(|o| o.zone == Zone::Graveyard)
            {
                arrive(state, source.object, you, owner_control, face, tapped);
                if let Some((kind, amount)) = counters
                    && (state.entries.contains(source.object)
                        || state
                            .object(source.object)
                            .is_some_and(|o| o.zone == Zone::Battlefield))
                {
                    crate::entry::put_entry_counters(state, source.object, kind, amount);
                }
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

/// CR 611.2e applies these changes as the card enters, not afterwards.
/// A return that never happens leaves neither tapped state nor modifiers.
fn return_modified(
    state: &mut GameState,
    res: &Resolution,
    target: TargetSpec,
    owner_control: bool,
    tapped: bool,
    counters: Option<(baylee_cards_dsl::CounterKind, u16)>,
    modifiers: &'static [baylee_cards_dsl::Modifier],
) {
    use baylee_cards_dsl::Duration;
    use baylee_core::ids::EffectId;
    for id in moving_objects(state, res, target) {
        let Some(object) = state
            .object(id)
            .filter(|o| o.zone == Zone::Graveyard && o.is_card())
        else {
            continue;
        };
        let previous = (
            object.controller,
            object.base_controller,
            object.kind,
            object.status.contains(crate::object::Status::TAPPED),
        );
        let graveyard_version = object.version;
        let version = graveyard_version.wrapping_add(1);
        let mut created = smallvec::SmallVec::<[EffectId; 4]>::new();
        for &modifier in modifiers {
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
        if tapped {
            state.set_tapped(id, true);
        }
        return_card(state, id, res.controller, owner_control, counters);
        if !state.object(id).is_some_and(|o| {
            (o.zone == Zone::Battlefield && o.version == version) || state.entries.contains(id)
        }) {
            state
                .effects
                .remove_where(|effect| created.contains(&effect.id));
            if state.object(id).is_some_and(|object| {
                object.zone == Zone::Graveyard && object.version == graveyard_version
            }) {
                state.restore_tapped(id, previous.3);
                let object = state.object_mut(id).expect("the same graveyard object");
                object.kind = previous.2;
                // This is a rollback, not a control change: preserve both the
                // projected controller and its previous layer-2 fallback.
                object.controller = previous.0;
                object.base_controller = previous.1;
            }
        }
    }
}

/// The shared arrival door for ordinary and characteristic-changing returns.
pub(super) fn return_card(
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
    if let Some((kind, n)) = counters {
        crate::entry::put_entry_counters(state, id, kind, n);
    }
}

/// Choose the entering face before any entry replacement or trigger can see it.
/// Entering transformed is not the transform action (CR 712.11a, 701.27a).
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
