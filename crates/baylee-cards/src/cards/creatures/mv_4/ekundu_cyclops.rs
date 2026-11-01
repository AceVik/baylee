//! Ekundu Cyclops — {3}{R} — Creature — Cyclops
//! Oracle: If a creature you control attacks, this creature also attacks if able.
//! Set: MIR #171 — Mirage | Scryfall ID: 9047d292-8f5c-4a6b-b74e-c8dbf3e0ab24 | Oracle ID: 6c89e4e6-c4d7-416c-b985-41739c6f5b50
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EKUNDU_CYCLOPS,
    oracle_id = "6c89e4e6-c4d7-416c-b985-41739c6f5b50",
    scryfall_id = "9047d292-8f5c-4a6b-b74e-c8dbf3e0ab24",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Ekundu Cyclops",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CYCLOPS],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
