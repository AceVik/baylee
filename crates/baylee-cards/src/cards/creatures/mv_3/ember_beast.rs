//! Ember Beast — {2}{R} — Creature — Beast
//! Oracle: This creature can't attack or block alone.
//! Set: DMR #117 — Dominaria Remastered | Scryfall ID: a75faca7-cfcf-4c3b-a507-c89659cce93a | Oracle ID: 58edb282-a124-47bc-b123-1c34b6804f48
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMBER_BEAST,
    oracle_id = "58edb282-a124-47bc-b123-1c34b6804f48",
    scryfall_id = "a75faca7-cfcf-4c3b-a507-c89659cce93a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Ember Beast",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
