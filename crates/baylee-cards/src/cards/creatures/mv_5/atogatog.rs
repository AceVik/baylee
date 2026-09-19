//! Atogatog — {W}{U}{B}{R}{G} — Legendary Creature — Atog
//! Oracle: Sacrifice an Atog creature: Atogatog gets +X/+X until end of turn, where X is the sacrificed creature's power.
//! Set: ODY #286 — Odyssey | Scryfall ID: 4a3e6eb5-6d0f-4f82-86f9-bbce8d27afbb | Oracle ID: b9fdb740-e5d7-4464-b378-2e0514ca28d8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ATOGATOG,
    oracle_id = "b9fdb740-e5d7-4464-b378-2e0514ca28d8",
    scryfall_id = "4a3e6eb5-6d0f-4f82-86f9-bbce8d27afbb",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Atogatog",
        mana_cost = mana!("{W}{U}{B}{R}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ATOG],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
