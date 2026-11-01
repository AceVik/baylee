//! Bazaar of Wonders — {3}{U}{U} — World Enchantment
//! Oracle: When this enchantment enters, exile all graveyards.
//! Oracle: Whenever a player casts a spell, counter it if a card with the same name is in a graveyard or a nontoken permanent with the same name is on the battlefield.
//! Set: MIR #55 — Mirage | Scryfall ID: 78e7a165-e135-4b85-943d-8352b6e65870 | Oracle ID: 7fca65b8-01fe-4858-97c6-f97ae82cd801
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BAZAAR_OF_WONDERS,
    oracle_id = "7fca65b8-01fe-4858-97c6-f97ae82cd801",
    scryfall_id = "78e7a165-e135-4b85-943d-8352b6e65870",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Bazaar of Wonders",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
