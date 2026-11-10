//! Hired Giant — {3}{R} — Creature — Giant
//! Oracle: When this creature enters, each other player may search their library for a land card and put that card onto the battlefield. Then each player who searched their library this way shuffles.
//! Set: MMQ #194 — Mercadian Masques | Scryfall ID: dc33920a-f05c-46fa-b94b-278af0022b78 | Oracle ID: 79c9a963-a428-45f1-a647-5a91ef64286e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIRED_GIANT,
    oracle_id = "79c9a963-a428-45f1-a647-5a91ef64286e",
    scryfall_id = "dc33920a-f05c-46fa-b94b-278af0022b78",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Hired Giant",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
