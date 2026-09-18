//! Dragonback Assault — {3}{G}{U}{R} — Enchantment
//! Oracle: When this enchantment enters, it deals 3 damage to each creature and each planeswalker.
//! Oracle: Landfall — Whenever a land you control enters, create a 4/4 red Dragon creature token with flying.
//! Set: TDM #179 — Tarkir: Dragonstorm | Scryfall ID: d54cc838-d79d-433a-99fb-d6e4d1c1431d | Oracle ID: 413fb2db-f1a1-4d22-ac37-a52821d35ca2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DRAGONBACK_ASSAULT,
    oracle_id = "413fb2db-f1a1-4d22-ac37-a52821d35ca2",
    scryfall_id = "d54cc838-d79d-433a-99fb-d6e4d1c1431d",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Dragonback Assault",
        mana_cost = mana!("{3}{G}{U}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
