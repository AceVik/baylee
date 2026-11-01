//! Jeweled Spirit — {3}{W}{W} — Creature — Spirit
//! Oracle: Flying
//! Oracle: Sacrifice two lands: This creature gains protection from artifacts or from the color of your choice until end of turn.
//! Set: PCY #12 — Prophecy | Scryfall ID: b0d3e681-bd4b-41e9-8db4-083172f3caad | Oracle ID: cddf2e46-79dc-4d6f-bb67-db55d966e8e2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JEWELED_SPIRIT,
    oracle_id = "cddf2e46-79dc-4d6f-bb67-db55d966e8e2",
    scryfall_id = "b0d3e681-bd4b-41e9-8db4-083172f3caad",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Jeweled Spirit",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
