//! Snapping Thragg — {4}{R} — Creature — Beast
//! Oracle: Whenever this creature deals combat damage to a player, you may have it deal 3 damage to target creature that player controls.
//! Oracle: Morph {4}{R}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #233 — Onslaught | Scryfall ID: c8a47d41-b893-46b9-90c9-ccd8f9f78855 | Oracle ID: 1a02cb6d-965a-4eeb-b126-8e153d4832a2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SNAPPING_THRAGG,
    oracle_id = "1a02cb6d-965a-4eeb-b126-8e153d4832a2",
    scryfall_id = "c8a47d41-b893-46b9-90c9-ccd8f9f78855",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Snapping Thragg",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
