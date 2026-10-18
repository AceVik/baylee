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
) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let source = subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))?;
    if !state
        .object(source.object)
        .is_some_and(|o| o.owner == you && o.controller == you)
    {
        return None;
    }
    let partners: Vec<_> = state
        .battlefield_seen()
        .filter(|&id| {
            id != source.object
                && state.object(id).is_some_and(|o| {
                    o.owner == you
                        && o.controller == you
                        && o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                        && state.names.get(o.characteristics().name) == partner_name
                })
        })
        .collect();
    let &partner = partners.first()?;
    if partners.len() > 1 {
        res.awaiting = Some(AwaitingOp::MeldPartner);
        return Some(Pending::ChooseCards {
            player: you,
            options: partners,
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
            total: None,
        });
    }
    finish(state, res, partner, source_name, partner_name, melded);
    None
}

pub(super) fn finish(
    state: &mut GameState,
    res: &Resolution,
    partner: ObjectId,
    source_name: &'static str,
    partner_name: &'static str,
    melded: &'static baylee_cards_dsl::TokenDef,
) {
    let you = res.controller;
    let Some(source) = subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))
    else {
        return;
    };
    // Names on the battlefield determine which objects are exiled. Only the
    // actual physical pair can meld afterwards; copies and tokens stay exiled.
    let physical_name_is = |id, name| {
        state
            .object(id)
            .filter(|object| {
                object.is_card()
                    && !object
                        .riders
                        .iter()
                        .any(|r| matches!(r, crate::object::Rider::MergedComponent(_)))
            })
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
    state
        .object_mut(partner)
        .expect("meld component")
        .riders
        .push(crate::object::Rider::MergedInto(source.object));
    let mana_value = [source.object, partner]
        .into_iter()
        .filter_map(|id| state.object(id))
        .map(|o| o.base.mana_value())
        .sum::<u32>();
    let mut base = state.token_base(melded, None);
    std::sync::Arc::make_mut(&mut base).front_mana_value =
        Some(u8::try_from(mana_value).expect("meld pair mana value fits"));
    // The combined back's ability is not a row printed on the front land.
    // Like an ordinary token program, its origin is resolved from `obj.token`.
    let abilities = crate::object::AbilityList::from_static(melded.abilities, None, None);
    if let Some(obj) = state.object_mut(source.object) {
        obj.kind = ObjectKind::Permanent;
        obj.set_controller(you);
        obj.original_base = Some(std::sync::Arc::clone(&obj.base));
        obj.base = base;
        obj.token = Some(melded);
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
