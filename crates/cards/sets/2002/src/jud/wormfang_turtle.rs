//! Wormfang Turtle — {2}{U} — Creature — Nightmare Turtle Beast
//! Oracle: When this creature enters, exile a land you control.
//! Oracle: When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
//! Set: JUD #60 — Judgment | Scryfall ID: 48404362-7579-4896-a71a-8eb40e5ac416 | Oracle ID: 771f9ba4-6c65-4497-ad48-869994435556
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WORMFANG_TURTLE,
    oracle_id = "771f9ba4-6c65-4497-ad48-869994435556",
    scryfall_id = "48404362-7579-4896-a71a-8eb40e5ac416",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Wormfang Turtle",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::NIGHTMARE,
            subtypes::creature::TURTLE,
            subtypes::creature::BEAST
        ],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
