//! Lavaborn Muse — {3}{R} — Creature — Spirit
//! Oracle: At the beginning of each opponent's upkeep, if that player has two or fewer cards in hand, this creature deals 3 damage to that player.
//! Set: DDK #50 — Duel Decks: Sorin vs. Tibalt | Scryfall ID: f9ac12fe-f895-4450-8516-1feb6a48f1bc | Oracle ID: 2825ff5a-5340-4494-bfde-80f6f5bce5a2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LAVABORN_MUSE,
    oracle_id = "2825ff5a-5340-4494-bfde-80f6f5bce5a2",
    scryfall_id = "f9ac12fe-f895-4450-8516-1feb6a48f1bc",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Lavaborn Muse",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
