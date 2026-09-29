//! Manabarbs — {3}{R} — Enchantment
//! Oracle: Whenever a player taps a land for mana, this enchantment deals 1 damage to that player.
//! Set: M12 #150 — Magic 2012 | Scryfall ID: adf081d5-e644-4f46-8bc8-a754b089acb4 | Oracle ID: 0f1afedd-c60f-454f-b84a-c8117aec0128
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANABARBS,
    oracle_id = "0f1afedd-c60f-454f-b84a-c8117aec0128",
    scryfall_id = "adf081d5-e644-4f46-8bc8-a754b089acb4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Manabarbs",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
