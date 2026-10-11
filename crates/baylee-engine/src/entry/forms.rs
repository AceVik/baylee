//! Entry-time numeric and copiable-form choices.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn question(entry: &Attempt, modifier: EnterModifier) -> Option<(Question, Pending)> {
    let player = entry.controller;
    match modifier {
        EnterModifier::ChooseNumber { min, max } => Some((
            Question::Number,
            Pending::ChooseNumber {
                player,
                min: u32::from(min),
                max: u32::from(max),
                reason: crate::choice::NumberPrompt::Choice,
            },
        )),
        EnterModifier::ChooseShape(choices) => Some((
            Question::Shape { choices },
            Pending::ChooseCastMode {
                player,
                object: entry.object,
                options: choices
                    .iter()
                    .enumerate()
                    .map(|(index, _)| crate::choice::CastModeDesc {
                        index: u32::try_from(index).expect("shape count fits"),
                        kind: crate::choice::CastModeKind::Mode(index),
                        cost: baylee_core::mana::ManaCost::ZERO,
                    })
                    .collect(),
            },
        )),
        _ => None,
    }
}

pub(super) fn answer(state: &mut GameState, question: &Question, action: &PlayerAction) -> bool {
    let Question::Shape { choices } = question else {
        return false;
    };
    let PlayerAction::ChooseMode(index) = action else {
        unreachable!("shape choice");
    };
    copying::apply_shape(state, choices[*index]);
    true
}
