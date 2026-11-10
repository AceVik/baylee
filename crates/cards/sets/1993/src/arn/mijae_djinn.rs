//! Mijae Djinn — {R}{R}{R} — Creature — Djinn
//! Oracle: Whenever this creature attacks, flip a coin. If you lose the flip, remove this creature from combat and tap it.
//! Set: ME4 #127 — Masters Edition IV | Scryfall ID: b4eb4f9e-4c11-4563-9f9a-87ec93274695 | Oracle ID: 61bebdfa-5df0-4952-abf9-dc5e0e4f57ea
// IMPLEMENTED — the attack trigger flips a coin (Effect::FlipCoin); a lost
// flip removes the Djinn itself from combat (untargeted
// Effect::RemoveTargetFromCombat) and taps it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIJAE_DJINN,
    oracle_id = "61bebdfa-5df0-4952-abf9-dc5e0e4f57ea",
    scryfall_id = "b4eb4f9e-4c11-4563-9f9a-87ec93274695",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mijae Djinn",
        mana_cost = mana!("{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(6),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::FlipCoin {
            won: &[],
            lost: &[
                Effect::RemoveTargetFromCombat { unblock: false },
                Effect::TapSelf,
            ],
        }]
    ),],
);
