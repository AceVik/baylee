//! Braingeyser — {X}{U}{U} — Sorcery
//! Oracle: Target player draws X cards.
//! Set: ME4 #40 — Masters Edition IV | Scryfall ID: 23b33d16-dbaa-4742-9317-eac745f772ac | Oracle ID: 9908e597-9470-4c13-8387-39431b380138
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRAINGEYSER,
    oracle_id = "9908e597-9470-4c13-8387-39431b380138",
    scryfall_id = "23b33d16-dbaa-4742-9317-eac745f772ac",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Braingeyser",
        mana_cost = mana!("{X}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
