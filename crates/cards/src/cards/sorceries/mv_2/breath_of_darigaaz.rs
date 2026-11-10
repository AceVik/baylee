//! Breath of Darigaaz — {1}{R} — Sorcery
//! Oracle: Kicker {2} (You may pay an additional {2} as you cast this spell.)
//! Oracle: Breath of Darigaaz deals 1 damage to each creature without flying and each player. If this spell was kicked, it deals 4 damage to each creature without flying and each player instead.
//! Set: CM2 #88 — Commander Anthology Volume II | Scryfall ID: cbb60f03-7bcf-46a3-9310-c2067d9c5396 | Oracle ID: 1954994f-17bf-4ea5-af72-60f9bfcb6569
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BREATH_OF_DARIGAAZ,
    oracle_id = "1954994f-17bf-4ea5-af72-60f9bfcb6569",
    scryfall_id = "cbb60f03-7bcf-46a3-9310-c2067d9c5396",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Breath of Darigaaz",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
