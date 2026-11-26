//! Resolving a meld pair (`CR 701.42`).

#[allow(clippy::wildcard_imports)] // the parent interpreter vocabulary
use super::*;
use crate::state::CardLookup as _;

/// Complete rules parameters for a meld instruction (`Effect::MeldWithNamed`
/// or `Effect::MeldPair`).
#[derive(Clone, Copy)]
pub(super) struct MeldSpec {
    pub(super) source_name: &'static str,
    pub(super) partner_name: &'static str,
    pub(super) partner_types: baylee_core::types::TypeSet,
    pub(super) require_attacking: bool,
    pub(super) enters_tapped: bool,
    pub(super) enters_attacking: bool,
    pub(super) melded: &'static baylee_cards_dsl::TokenDef,
}

impl MeldSpec {
    pub(super) fn from_effect(effect: Effect) -> Option<Self> {
        match effect {
            Effect::MeldWithNamed {
                source_name,
                partner_name,
                melded,
            } => Some(Self {
                source_name,
                partner_name,
                partner_types: baylee_core::types::TypeSet::CREATURE,
                require_attacking: false,
                enters_tapped: false,
                enters_attacking: false,
                melded,
            }),
            Effect::MeldPair {
                source_name,
                partner_name,
                partner_types,
                require_attacking,
                enters_tapped,
                enters_attacking,
                melded,
            } => Some(Self {
                source_name,
                partner_name,
                partner_types,
                require_attacking,
                enters_tapped,
                enters_attacking,
                melded,
            }),
            _ => None,
        }
    }
}

pub(super) fn meld(state: &mut GameState, res: &mut Resolution, spec: MeldSpec) -> Option<Pending> {
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
    if spec.require_attacking && !state.combat.is_attacking(source.object) {
        return None;
    }
    let partners: Vec<_> = state
        .battlefield_seen()
        .filter(|&id| {
            id != source.object
                && (!spec.require_attacking || state.combat.is_attacking(id))
                && state.object(id).is_some_and(|o| {
                    o.owner == you
                        && o.controller == you
                        && (spec.partner_types.is_empty()
                            || o.characteristics().types.intersects(spec.partner_types))
                        && state.names.get(o.characteristics().name) == spec.partner_name
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
    finish(state, res, partner, spec)
}

pub(super) fn finish(
    state: &mut GameState,
    res: &mut Resolution,
    partner: ObjectId,
    spec: MeldSpec,
) -> Option<Pending> {
    let you = res.controller;
    let source = subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))?;
    // Names on the battlefield determine which objects are exiled. Only the
    // actual physical pair can meld afterwards; copies and tokens stay exiled
    // (CR 701.42b).
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
    let can_meld = physical_name_is(source.object, spec.source_name)
        && physical_name_is(partner, spec.partner_name);
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
        return None;
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
    let mut base = state.token_base(spec.melded, None);
    std::sync::Arc::make_mut(&mut base).front_mana_value =
        Some(u8::try_from(mana_value).expect("meld pair mana value fits"));
    // The combined back's abilities are not printed on the front card.
    // Like an ordinary token program, its origin is resolved from `obj.token`.
    let abilities = crate::object::AbilityList::from_static(spec.melded.abilities, None, None);
    if let Some(obj) = state.object_mut(source.object) {
        obj.kind = ObjectKind::Permanent;
        obj.set_controller(you);
        obj.original_base = Some(std::sync::Arc::clone(&obj.base));
        obj.base = base;
        obj.token = Some(spec.melded);
        obj.take_abilities(abilities);
        obj.riders.push(crate::object::Rider::EntryCopyApplied);
        obj.riders
            .push(crate::object::Rider::MergedComponent(partner));
        obj.cache.clear();
    }
    state.set_tapped(source.object, spec.enters_tapped || spec.enters_attacking);
    let _ = state.move_object(
        source.object,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
    if spec.enters_attacking {
        return super::attacking_tokens::start_for_active(state, res, vec![source.object]);
    }
    None
}
