//! El-Hajjâj — {1}{B}{B} — Creature — Human Wizard
//! Oracle: Whenever this creature deals damage, you gain that much life.
//! Set: 4ED #134 — Fourth Edition | Scryfall ID: cf371bd2-89ad-487e-8f27-37a6e75ca0f5 | Oracle ID: f92c9a5d-853f-4157-89ba-8d8c8033c533
// IMPLEMENTED — whenever it deals damage (Trigger::DealsDamage: each damage
// event, a combat damage step once), its controller gains that much life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EL_HAJJAJ,
    oracle_id = "f92c9a5d-853f-4157-89ba-8d8c8033c533",
    scryfall_id = "cf371bd2-89ad-487e-8f27-37a6e75ca0f5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "El-Hajjâj",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::DealsDamage(&Filter::This),
        &[Effect::GainLife {
            amount: Amount::EventAmount
        }]
    )],
);
