//! Tephraderm — {4}{R} — Creature — Beast
//! Oracle: Whenever a creature deals damage to this creature, this creature deals that much damage to that creature.
//! Oracle: Whenever a spell deals damage to this creature, this creature deals that much damage to that spell's controller.
//! Set: ONS #239 — Onslaught | Scryfall ID: 41b65eba-140b-4c1d-b796-8134b7c1ede8 | Oracle ID: 57d411c6-68d8-441d-885f-17412f6503c2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TEPHRADERM,
    oracle_id = "57d411c6-68d8-441d-885f-17412f6503c2",
    scryfall_id = "41b65eba-140b-4c1d-b796-8134b7c1ede8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Tephraderm",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
