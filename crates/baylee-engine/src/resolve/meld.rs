//! Resolving a meld pair.

#[allow(clippy::wildcard_imports)] // the parent interpreter vocabulary
use super::*;
use crate::state::CardLookup as _;

pub(super) fn meld_with_named(
    state: &mut GameState,
    res: &mut Resolution,
    source_name: &'static str,
    partner_name: &'static str,
    melded: &'static baylee_cards_dsl::TokenDef,
) {
    let you = res.controller;
    if state.has_left(you) {
        return;
    }
    let Some(source) = subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))
    else {
        return;
    };
    if !state
        .object(source.object)
        .is_some_and(|o| o.owner == you && o.controller == you)
    {
        return;
    }
    let Some(partner) = state.battlefield_seen().find(|&id| {
        id != source.object
            && state.object(id).is_some_and(|o| {
                o.owner == you
                    && o.controller == you
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
                    && state.names.get(o.characteristics().name) == partner_name
            })
    }) else {
        return;
    };
    // Names on the battlefield determine which objects are exiled. Only the
    // actual physical pair can meld afterwards; copies and tokens stay exiled.
    let physical_name_is = |id, name| {
        state
            .object(id)
            .and_then(|object| object.card)
            .and_then(|card| state.bases.card(card.index))
            .is_some_and(|definition| definition.name() == name)
    };
    let can_meld =
        physical_name_is(source.object, source_name) && physical_name_is(partner, partner_name);
    if let Some(obj) = state.object_mut(source.object) {
        obj.kind = ObjectKind::Card;
    }
    if let Some(obj) = state.object_mut(partner) {
        obj.kind = ObjectKind::Card;
    }
    let _ = state.move_object(
        source.object,
        ZoneLocation::Exile(you),
        ZonePosition::Top,
        Cause::Effect,
    );
    let _ = state.move_object(
        partner,
        ZoneLocation::Exile(you),
        ZonePosition::Top,
        Cause::Effect,
    );
    if !can_meld
        || !state
            .object(source.object)
            .is_some_and(|o| o.zone == crate::zone::Zone::Exile)
        || !state
            .object(partner)
            .is_some_and(|o| o.zone == crate::zone::Zone::Exile)
    {
        return;
    }
    let _ = state.move_object(
        partner,
        ZoneLocation::Command(you),
        ZonePosition::Top,
        Cause::Effect,
    );
    let base = state.token_base(melded, None);
    let printed = state
        .object(source.object)
        .and_then(GameObject::printed_face);
    let abilities = crate::object::AbilityList::from_static(melded.abilities, printed, None);
    if let Some(obj) = state.object_mut(source.object) {
        obj.kind = ObjectKind::Permanent;
        obj.set_controller(you);
        obj.original_base = Some(std::sync::Arc::clone(&obj.base));
        obj.base = base;
        obj.take_abilities(abilities);
        obj.riders.push(crate::object::Rider::EntryCopyApplied);
        obj.riders
            .push(crate::object::Rider::MergedComponent(partner));
        obj.cache.clear();
    }
    state.set_tapped(source.object, false);
    let _ = state.move_object(
        source.object,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
}

