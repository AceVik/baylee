//! Resurrection — {2}{W}{W} — Sorcery
//! Oracle: Return target creature card from your graveyard to the battlefield.
//! Set: UMA #30 — Ultimate Masters | Scryfall ID: a176b295-9406-4d6b-b15c-e81a72e66874 | Oracle ID: 837417d8-8260-486d-a3ed-3b5711eaf34a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RESURRECTION,
    oracle_id = "837417d8-8260-486d-a3ed-3b5711eaf34a",
    scryfall_id = "a176b295-9406-4d6b-b15c-e81a72e66874",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Resurrection",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
