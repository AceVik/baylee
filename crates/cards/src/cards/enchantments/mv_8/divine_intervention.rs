//! Divine Intervention — {6}{W}{W} — Enchantment
//! Oracle: This enchantment enters with two intervention counters on it.
//! Oracle: At the beginning of your upkeep, remove an intervention counter from this enchantment.
//! Oracle: When you remove the last intervention counter from this enchantment, the game is a draw.
//! Set: ME3 #8 — Masters Edition III | Scryfall ID: e250897e-a875-4c3c-bfec-4a09143dd587 | Oracle ID: 0158a440-6573-4f12-958b-a5f0cf190d5b
// IMPLEMENTED — enters with two intervention counters, removes one at the
// beginning of your upkeep, and the game is a draw when you remove the last.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIVINE_INTERVENTION,
    oracle_id = "0158a440-6573-4f12-958b-a5f0cf190d5b",
    scryfall_id = "e250897e-a875-4c3c-bfec-4a09143dd587",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Divine Intervention",
        mana_cost = mana!("{6}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::INTERVENTION,
            amount: Amount::Fixed(2),
        }],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::RemoveCounterSelf {
                kind: counters::INTERVENTION,
                n: 1,
            }],
        ),
        triggered!(
            Trigger::LastCounterRemoved {
                kind: counters::INTERVENTION,
            },
            &[Effect::DrawGame],
        ),
    ],
);
