//! Casting card copies during the parent resolution (CR 707.12).
use super::{Cause, GameState, Pending, Resolution, ZoneLocation, ZonePosition};
use crate::object::{GameObject, ObjectKind, Rider};
use crate::zone::Zone;
use baylee_cards_dsl::Filter;

pub(super) fn exile_and_cast(
    state: &mut GameState,
    res: &mut Resolution,
    graveyard: Option<&'static Filter>,
) -> Option<Pending> {
    let cards: Vec<_> = if let Some(filter) = graveyard {
        state
            .zones
            .list(ZoneLocation::Graveyard(res.controller))
            .iter()
            .copied()
            .filter(|id| {
                state.object(*id).is_some_and(|object| {
                    crate::eval::matches_with_context(
                        filter,
                        state,
                        object,
                        res.controller,
                        res.rule_context(),
                    )
                })
            })
            .collect()
    } else {
        res.targets.iter().copied().collect()
    };
    let mut copies = Vec::with_capacity(cards.len());
    for card in cards {
        let Some(owner) = state
            .object(card)
            .filter(|object| object.zone == Zone::Graveyard)
            .map(|object| object.owner)
        else {
            continue;
        };
        if state
            .move_object(
                card,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            )
            .is_err()
        {
            continue;
        }
        let Some(original) = state
            .object(card)
            .filter(|object| object.zone == Zone::Exile)
        else {
            continue;
        };
        // Only copiable values survive; continuous color/type effects are reapplied.
        let base = (*original.base).clone();
        let card_ref = original.card;
        let face_index = original.face_index;
        let timestamp = state.next_timestamp();
        let copy = state.arena.insert_with(|id| {
            let mut object = GameObject::new_bare(id, res.controller, ObjectKind::Card, base);
            object.timestamp = timestamp;
            object.controlled_since = timestamp;
            object.zone = Zone::Exile;
            object.zone_owner = Some(res.controller);
            object.card = card_ref;
            object.face_index = face_index;
            object.riders.push(Rider::SpellCopy);
            object
        });
        state.zones.insert(
            copy,
            ZoneLocation::Exile(res.controller),
            ZonePosition::Top,
            true,
        );
        state.watch_token_cleanup(copy);
        copies.push(state.source_identity(copy).expect("new card copy"));
    }
    state.invalidate_projections();
    super::hidden_exile::continue_play(state, res, copies, false)
}
