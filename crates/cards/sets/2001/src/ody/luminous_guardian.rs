//! Luminous Guardian — {3}{W} — Creature — Human Nomad
//! Oracle: {W}: This creature gets +0/+1 until end of turn.
//! Oracle: {2}: This creature can block an additional creature this turn.
//! Set: ODY #31 — Odyssey | Scryfall ID: a5becc51-4df8-4f67-a029-42a3da598a26 | Oracle ID: 0d9a31f5-20d0-4266-95ec-38b4fa96b8aa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LUMINOUS_GUARDIAN,
    oracle_id = "0d9a31f5-20d0-4266-95ec-38b4fa96b8aa",
    scryfall_id = "a5becc51-4df8-4f67-a029-42a3da598a26",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Luminous Guardian",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOMAD],
        power = Some(1),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
