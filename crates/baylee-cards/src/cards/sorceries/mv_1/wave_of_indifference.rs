//! Wave of Indifference — {X}{R} — Sorcery
//! Oracle: X target creatures can't block this turn.
//! Set: ONS #243 — Onslaught | Scryfall ID: 2c88b942-06d5-45d8-a4d8-6ca864f65516 | Oracle ID: 932a47fa-9b78-4c7b-8df8-65e0309d663c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAVE_OF_INDIFFERENCE,
    oracle_id = "932a47fa-9b78-4c7b-8df8-65e0309d663c",
    scryfall_id = "2c88b942-06d5-45d8-a4d8-6ca864f65516",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Wave of Indifference",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
