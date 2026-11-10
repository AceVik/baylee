//! Skullscorch — {R}{R} — Sorcery
//! Oracle: Target player discards two cards at random unless that player has Skullscorch deal 4 damage to them.
//! Set: TOR #114 — Torment | Scryfall ID: 88f1343c-77bf-4f44-8226-fdfb2c2c7015 | Oracle ID: b52788ff-9956-400a-bf26-17e58f673af0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SKULLSCORCH,
    oracle_id = "b52788ff-9956-400a-bf26-17e58f673af0",
    scryfall_id = "88f1343c-77bf-4f44-8226-fdfb2c2c7015",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Skullscorch",
        mana_cost = mana!("{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
