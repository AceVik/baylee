//! Natural Selection — {G} — Instant
//! Oracle: Look at the top three cards of target player's library, then put them back in any order. You may have that player shuffle.
//! Set: 2ED #213 — Unlimited Edition | Scryfall ID: 315a6bfb-5417-465f-97d9-e157f5c3cf79 | Oracle ID: 57f90b30-bcb0-447e-8788-5c5ded187207
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NATURAL_SELECTION,
    oracle_id = "57f90b30-bcb0-447e-8788-5c5ded187207",
    scryfall_id = "315a6bfb-5417-465f-97d9-e157f5c3cf79",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Natural Selection",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
