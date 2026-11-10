//! Zebra Unicorn — {2}{G}{W} — Creature — Unicorn
//! Oracle: Whenever this creature deals damage, you gain that much life.
//! Set: MIR #290 — Mirage | Scryfall ID: a663ee9d-78f1-4c89-af9e-c788e165fa91 | Oracle ID: 2f91818f-a6fe-4472-b52b-ab574f790905
// IMPLEMENTED — whenever this creature deals damage, you gain that much life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZEBRA_UNICORN,
    oracle_id = "2f91818f-a6fe-4472-b52b-ab574f790905",
    scryfall_id = "a663ee9d-78f1-4c89-af9e-c788e165fa91",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Zebra Unicorn",
        mana_cost = mana!("{2}{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::UNICORN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamage(&Filter::This),
        &[Effect::GainLife {
            amount: Amount::EventAmount,
        }],
    )],
);
