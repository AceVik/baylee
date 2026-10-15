//! Conservative bounds for the loop detector's creature-arrival history.
//!
//! Unknown vocabulary keeps the exact count. This deliberately small proof
//! recognizes only instructions whose operands cannot observe that count and
//! direct lower-bound conditions; extending it is an optimization, not required
//! for correct card evaluation. Snapshot hashing never uses this normalization.

use crate::state::{CardLookup, GameState};
use baylee_cards_dsl::{
    AbilityDef, ActivationX, Amount, Condition, Effect, EnterModifier, Trigger,
};

pub(super) fn bound(state: &GameState) -> Option<u32> {
    if !state.effects.is_empty()
        || !state.delayed.is_empty()
        || !state.reflexive.is_empty()
        || !state.replacement_rules.is_empty()
        || !state.granted_actions.is_empty()
    {
        return None;
    }
    let mut bound = 0;
    for (_, object) in state.arena.iter() {
        // Include hidden and alternate-face text: a later choice can expose it.
        if let Some(card) = object.card {
            let definition = state.bases.card(card.index)?;
            for ability in definition.abilities {
                bound = bound.max(ability_bound(ability)?);
            }
            for face in definition.faces {
                if face.cost_reduction.is_some()
                    || face.enter_modifiers.iter().any(|modifier| {
                        !matches!(
                            modifier,
                            EnterModifier::WithCounters {
                                amount: Amount::Fixed(_),
                                ..
                            }
                        )
                    })
                {
                    return None;
                }
                for ability in face.abilities {
                    bound = bound.max(ability_bound(ability)?);
                }
            }
        }
        for ability in &object.printed_abilities(state.bases.as_ref()) {
            bound = bound.max(ability_bound(ability)?);
        }
    }
    for (_, list) in &state.ltb_abilities {
        for ability in &list.abilities {
            bound = bound.max(ability_bound(ability)?);
        }
    }
    Some(bound)
}

fn ability_bound(ability: &AbilityDef) -> Option<u32> {
    let (effects, condition) = match ability {
        AbilityDef::Spell {
            effects, condition, ..
        }
        | AbilityDef::Triggered {
            effects,
            condition,
            trigger: Trigger::StepBegin { .. } | Trigger::EntersBattlefield(_),
            ..
        } => (*effects, *condition),
        AbilityDef::Activated {
            effects,
            cost_reduction: None,
            x_value: ActivationX::Any | ActivationX::AtLeast(_),
            ..
        } => (*effects, None),
        AbilityDef::ActivatedConditional {
            effects,
            condition,
            cost_reduction: None,
            x_value: ActivationX::Any | ActivationX::AtLeast(_),
            ..
        } => (*effects, Some(*condition)),
        _ => return None,
    };
    // Defined activation X can read history before these effects resolve, so
    // only the fixed announcement constraints above are proven independent.
    // No nested instructions, amount-bearing targets, or granted abilities in
    // these shapes. An unfamiliar effect, including a new dynamic Amount use,
    // safely disables normalization for this game state.
    if !effects.iter().all(|effect| {
        matches!(
            effect,
            Effect::Blink { .. }
                | Effect::MoveCounter { .. }
                | Effect::AddMana {
                    amount: Amount::Fixed(_),
                    ..
                }
        )
    }) {
        return None;
    }
    match condition {
        None => Some(0),
        Some(Condition::AmountAtLeast(Amount::CreaturesEnteredUnderYourControlThisTurn, n)) => {
            Some(n)
        }
        _ => None,
    }
}
