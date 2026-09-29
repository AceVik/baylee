//! Psionic Blast — {2}{U} — Instant
//! Oracle: Psionic Blast deals 4 damage to any target and 2 damage to you.
//! Set: TSB #30 — Time Spiral Timeshifted | Scryfall ID: befb1728-4af3-420a-a51e-7fa95d2e7a00 | Oracle ID: 7f221ad6-7ec4-483d-a6b5-1456c95c1cad
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PSIONIC_BLAST,
    oracle_id = "7f221ad6-7ec4-483d-a6b5-1456c95c1cad",
    scryfall_id = "befb1728-4af3-420a-a51e-7fa95d2e7a00",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Psionic Blast",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
