//! Whipgrass Entangler — {2}{W} — Creature — Human Cleric
//! Oracle: {1}{W}: Until end of turn, target creature gains "This creature can't attack or block unless its controller pays {1} for each Cleric on the battlefield."
//! Set: LGN #26 — Legions | Scryfall ID: c0b18b09-b1ff-479d-bd1c-cb8620a34fe4 | Oracle ID: 098bbee7-df65-4167-9c65-ffa14fc0c6cc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHIPGRASS_ENTANGLER,
    oracle_id = "098bbee7-df65-4167-9c65-ffa14fc0c6cc",
    scryfall_id = "c0b18b09-b1ff-479d-bd1c-cb8620a34fe4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Whipgrass Entangler",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
