//! Chainflinger — {3}{R} — Creature — Beast
//! Oracle: {1}{R}, {T}: This creature deals 1 damage to any target.
//! Oracle: Threshold — {2}{R}, {T}: This creature deals 2 damage to any target. Activate only if there are seven or more cards in your graveyard.
//! Set: ODY #181 — Odyssey | Scryfall ID: 670a5bba-a10f-41f6-88cd-cef1dfe4bfa9 | Oracle ID: cfaf4871-2157-495d-bca8-d169bd1f7ae3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHAINFLINGER,
    oracle_id = "cfaf4871-2157-495d-bca8-d169bd1f7ae3",
    scryfall_id = "670a5bba-a10f-41f6-88cd-cef1dfe4bfa9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Chainflinger",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
