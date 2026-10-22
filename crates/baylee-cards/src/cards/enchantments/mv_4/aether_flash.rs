//! Aether Flash — {2}{R}{R} — Enchantment
//! Oracle: Whenever a creature enters, this enchantment deals 2 damage to it.
//! Set: 7ED #172 — Seventh Edition | Scryfall ID: 780f9197-e910-4c7a-bb4b-2c4a94903c39 | Oracle ID: a3c35742-e306-49b6-b042-db4f685c6f86
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AETHER_FLASH,
    oracle_id = "a3c35742-e306-49b6-b042-db4f685c6f86",
    scryfall_id = "780f9197-e910-4c7a-bb4b-2c4a94903c39",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aether Flash",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&Filter::CREATURE),
        &[Effect::DealDamageEach {
            amount: Amount::Fixed(2),
            filter: &Filter::Reference,
        }],
    )],
);
