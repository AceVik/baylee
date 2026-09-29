//! Spell Blast — {X}{U} — Instant
//! Oracle: Counter target spell with mana value X. (For example, if that spell's mana cost is {3}{U}{U}, X is 5.)
//! Set: TPR #69 — Tempest Remastered | Scryfall ID: 26fb30a8-9f6b-425b-95d3-0719319dbb3f | Oracle ID: 04477339-7ed5-4770-9e5c-6e481ffcc858
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPELL_BLAST,
    oracle_id = "04477339-7ed5-4770-9e5c-6e481ffcc858",
    scryfall_id = "26fb30a8-9f6b-425b-95d3-0719319dbb3f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Spell Blast",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
