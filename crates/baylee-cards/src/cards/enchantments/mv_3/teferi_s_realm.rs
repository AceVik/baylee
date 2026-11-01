//! Teferi's Realm — {1}{U}{U} — World Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses artifact, creature, land, or non-Aura enchantment. All nontoken permanents of that type phase out. (While they're phased out, they're treated as though they don't exist. Each one phases in before its controller untaps during their next untap step.)
//! Set: VIS #44 — Visions | Scryfall ID: aba3e4ea-2241-4f1e-a46b-70f512fe729e | Oracle ID: 9851d934-2e07-49c8-b08b-15f96d0f3f0c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEFERI_S_REALM,
    oracle_id = "9851d934-2e07-49c8-b08b-15f96d0f3f0c",
    scryfall_id = "aba3e4ea-2241-4f1e-a46b-70f512fe729e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Teferi's Realm",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
