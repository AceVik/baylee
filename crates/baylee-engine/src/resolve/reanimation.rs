//! Returning cards to the battlefield, including double-faced returns.
use super::zones::moving_objects;
#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

#[allow(clippy::too_many_lines)] // dispatch of the unchanged return operations
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
        Effect::ExileSelfReturnAsFace {
            face,
            owner_control,
        } => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            // Blink's arrival, for the same reasons: nothing is put onto the
            // battlefield under a player who has left (CR 800.4b), and the
            // card that comes back is a new object (CR 400.7) that enters
            // under the control the sentence names, as its default and
            // written before it arrives. It always returned under its owner,
            // so a stolen Fable of the Mirror-Breaker went home at chapter
            // III where the card says "under your control".
            if !owner_control && state.has_left(you) {
                return None;
            }
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Permanent;
                obj.set_controller(if owner_control { owner } else { you });
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            );
            if let Some(obj) = state.object_mut(res.source) {
                // The face switch needs the card definition (lookup);
                // finish_resolution applies it.
                obj.pending_face_change = Some(face);
            }
            None
        }
        _ => unreachable!("only return operations are dispatched here"),
    }
}
