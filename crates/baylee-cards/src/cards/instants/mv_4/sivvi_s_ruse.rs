//! Sivvi's Ruse — {2}{W}{W} — Instant
//! Oracle: If an opponent controls a Mountain and you control a Plains, you may cast this spell without paying its mana cost.
//! Oracle: Prevent all damage that would be dealt this turn to creatures you control.
//! Set: NEM #21 — Nemesis | Scryfall ID: 132112a0-0fb0-4a80-927d-39d34cf10159 | Oracle ID: c5edfaae-3ecd-41ad-970f-8cecb09bf1e4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SIVVI_S_RUSE,
    oracle_id = "c5edfaae-3ecd-41ad-970f-8cecb09bf1e4",
    scryfall_id = "132112a0-0fb0-4a80-927d-39d34cf10159",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sivvi's Ruse",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
