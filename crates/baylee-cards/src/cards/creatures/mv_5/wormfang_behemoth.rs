//! Wormfang Behemoth — {3}{U}{U} — Creature — Nightmare Fish Beast
//! Oracle: When this creature enters, exile all cards from your hand.
//! Oracle: When this creature leaves the battlefield, return the exiled cards to their owner's hand.
//! Set: JUD #55 — Judgment | Scryfall ID: 1c7f29aa-c069-4adb-b313-6a56849905d4 | Oracle ID: 77a000de-76d9-44f8-9789-875aa9f576e7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WORMFANG_BEHEMOTH,
    oracle_id = "77a000de-76d9-44f8-9789-875aa9f576e7",
    scryfall_id = "1c7f29aa-c069-4adb-b313-6a56849905d4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Wormfang Behemoth",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::NIGHTMARE,
            subtypes::creature::FISH,
            subtypes::creature::BEAST
        ],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
