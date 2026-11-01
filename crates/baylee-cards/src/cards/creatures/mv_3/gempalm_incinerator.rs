//! Gempalm Incinerator — {2}{R} — Creature — Goblin
//! Oracle: Cycling {1}{R} ({1}{R}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may have it deal X damage to target creature, where X is the number of Goblins on the battlefield.
//! Set: PLST #DMR-122 — The List | Scryfall ID: 0ac7667c-2526-4f4b-8886-2516d4a8fbae | Oracle ID: 467ea3e3-7767-405a-9d1a-8fa5de696376
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GEMPALM_INCINERATOR,
    oracle_id = "467ea3e3-7767-405a-9d1a-8fa5de696376",
    scryfall_id = "0ac7667c-2526-4f4b-8886-2516d4a8fbae",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Gempalm Incinerator",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
