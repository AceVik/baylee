//! Urborg Phantom — {2}{B} — Creature — Spirit Minion
//! Oracle: This creature can't block.
//! Oracle: {U}: Prevent all combat damage that would be dealt to and dealt by this creature this turn.
//! Set: INV #132 — Invasion | Scryfall ID: 397355b9-5b67-4973-972e-3505c500d116 | Oracle ID: f8b5d2f4-58a8-408c-bb67-6b5f9ba5b678
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URBORG_PHANTOM,
    oracle_id = "f8b5d2f4-58a8-408c-bb67-6b5f9ba5b678",
    scryfall_id = "397355b9-5b67-4973-972e-3505c500d116",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Urborg Phantom",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT, subtypes::creature::MINION],
        power = Some(3),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
