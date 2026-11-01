//! Grollub — {2}{B} — Creature — Beast
//! Oracle: Whenever this creature is dealt damage, each opponent gains that much life.
//! Set: EXO #63 — Exodus | Scryfall ID: 47f6301a-d581-4aaf-9993-3013323074aa | Oracle ID: 22c8c952-2713-418e-8259-aea09b4e740b
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GROLLUB,
    oracle_id = "22c8c952-2713-418e-8259-aea09b4e740b",
    scryfall_id = "47f6301a-d581-4aaf-9993-3013323074aa",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Grollub",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::DealtDamage(&Filter::This),
        &[Effect::GainLifeFor {
            amount: Amount::EventAmount,
            who: PlayerRel::EachOpponent,
        }],
    )],
);
