//! Rupture — {2}{R} — Sorcery
//! Oracle: Sacrifice a creature. Rupture deals damage equal to that creature's power to each creature without flying and each player.
//! Set: NEM #97 — Nemesis | Scryfall ID: db53c1fb-3641-44a3-b0b4-b7b2ba993646 | Oracle ID: d2179a54-4b45-4b67-a31c-aa1a7bcfda2c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUPTURE,
    oracle_id = "d2179a54-4b45-4b67-a31c-aa1a7bcfda2c",
    scryfall_id = "db53c1fb-3641-44a3-b0b4-b7b2ba993646",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Rupture",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
