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
            // Every target and not the first: see [`spec_objects`].
            //
            // CR 400.7: a card that has moved is a new object with no
            // relation to the one the effect named. A reanimation *spell* is
            // held to that by target legality (CR 608.2b), so the zone check
            // looks redundant beside the fifteen cards that cast one — but
            // undying and persist target nothing at all
            // (`TargetSpec::EventObject` is the creature that died), and
            // without it a card exiled in response to the keyword trigger
            // came back onto the battlefield out of exile.
            // "Under your control" when `you` has left the game: the card
            // stays where it is (CR 800.4b). Under its owner's it goes, since
            // an owner who had left would have taken the card with them.
            if !owner_control && state.has_left(you) {
                return None;
            }
            for target_id in moving_objects(state, res, target) {
                // The card has to still be in a graveyard, asked per card:
                // one of several targets leaving does not stop the others.
                if !state
                    .object(target_id)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard)
                {
                    continue;
                }
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Permanent;
                    let to = if owner_control { obj.owner } else { you };
                    obj.set_controller(to);
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
                // After the move and not before it: `move_object` clears
                // every field only a permanent can have, counters among
                // them, so a counter written first would be wiped on the
                // way in. `put_counters` and not `Counters::add`, because
                // "returns with a counter on it" is a counter an effect
                // puts on a permanent and a doubler has its say
                // (CR 614.16) — the same door `EnterModifier::WithCounters`
                // goes through.
                if let Some((kind, n)) = counters
                    && n > 0
                    && state
                        .object(target_id)
                        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                {
                    crate::replacement::put_counters(state, target_id, kind, n);
                }
            }
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
