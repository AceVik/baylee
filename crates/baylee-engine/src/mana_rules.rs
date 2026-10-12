//! Shared activation life prices and actual mana-type replacements.
use crate::{effects, object::GameObject, state::GameState};
use baylee_cards_dsl::Modifier;
use baylee_core::{ids::ObjectId, mana::ManaColor};

pub(crate) fn life_price(state: &GameState, source: ObjectId) -> u32 {
    if !state.effects.may_tax_mana_life() {
        return 0;
    }
    let Some(object) = state.object(source) else {
        return 0;
    };
    state
        .effects
        .iter()
        .filter_map(|effect| {
            if let Modifier::ManaAbilitiesCostLife(amount) = effect.modifier
                && effects::applies_to(state, effect, object)
            {
                Some(amount)
            } else {
                None
            }
        })
        .fold(0, u32::saturating_add)
}

/// Caller establishes that this production is a tap-symbol mana activation.
pub(crate) fn replace(state: &GameState, object: &GameObject, mut color: ManaColor) -> ManaColor {
    if !state.effects.may_replace_mana() {
        return color;
    }
    for effect in state.effects.iter() {
        if let Modifier::ReplaceTappedMana { from, to } =
            state.effect_text(effect).modifier(effect.modifier)
            && color == from
            && effects::applies_to(state, effect, object)
        {
            color = to;
        }
    }
    color
}

pub(crate) fn replace_types(state: &GameState, object: &GameObject, types: u8) -> u8 {
    if !state.effects.may_replace_mana() {
        return types;
    }
    ManaColor::ALL
        .into_iter()
        .filter(|color| types & (1 << color.index()) != 0)
        .fold(0, |types, color| {
            types | (1 << replace(state, object, color).index())
        })
}
