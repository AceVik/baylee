//! Enchanted Being — {1}{W}{W} — Creature — Human
//! Oracle: Prevent all combat damage that would be dealt to this creature by enchanted creatures.
//! Set: LEG #12 — Legends | Scryfall ID: 94c2880d-b37a-43ea-9fee-cd5a8ed75a7e | Oracle ID: c98b725e-ca16-4576-bf53-653d4028d861

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENCHANTED_BEING,
    oracle_id = "c98b725e-ca16-4576-bf53-653d4028d861",
    scryfall_id = "94c2880d-b37a-43ea-9fee-cd5a8ed75a7e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Enchanted Being",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::This,
        Modifier::PreventCombatDamageFrom(&Filter::And(&[Filter::CREATURE, Filter::Enchanted]))
    )],
);
