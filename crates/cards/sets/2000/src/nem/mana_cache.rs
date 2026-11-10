//! Mana Cache — {1}{R}{R} — Enchantment
//! Oracle: At the beginning of each player's end step, put a charge counter on this enchantment for each untapped land that player controls.
//! Oracle: Remove a charge counter from this enchantment: Add {C}. Any player may activate this ability but only during their turn before the end step.
//! Set: NEM #92 — Nemesis | Scryfall ID: 583a33b3-7833-48e5-88c3-849a5771ef6e | Oracle ID: 1c09f2d1-413b-4a92-aa39-99c1a2106814
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_CACHE,
    oracle_id = "1c09f2d1-413b-4a92-aa39-99c1a2106814",
    scryfall_id = "583a33b3-7833-48e5-88c3-849a5771ef6e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mana Cache",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
