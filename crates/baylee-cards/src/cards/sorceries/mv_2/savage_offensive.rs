//! Savage Offensive — {1}{R} — Sorcery
//! Oracle: Kicker {G} (You may pay an additional {G} as you cast this spell.)
//! Oracle: Creatures you control gain first strike until end of turn. If this spell was kicked, they get +1/+1 until end of turn.
//! Set: INV #162 — Invasion | Scryfall ID: 356744f3-e444-4f4e-bf00-80bb6b2ef76f | Oracle ID: b5944ca8-d54e-4830-9f66-be8fe97d101d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SAVAGE_OFFENSIVE,
    oracle_id = "b5944ca8-d54e-4830-9f66-be8fe97d101d",
    scryfall_id = "356744f3-e444-4f4e-bf00-80bb6b2ef76f",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Savage Offensive",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
