//! Dwarven Landslide — {3}{R} — Sorcery
//! Oracle: Kicker—{2}{R}, Sacrifice a land. (You may pay {2}{R} and sacrifice a land in addition to any other costs as you cast this spell.)
//! Oracle: Destroy target land. If this spell was kicked, destroy another target land.
//! Set: APC #60 — Apocalypse | Scryfall ID: 48ab243e-d08d-4ece-9725-4bb5f67b1c92 | Oracle ID: 5231a7d6-b6cf-4fa8-8da8-f0ecf023e054
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DWARVEN_LANDSLIDE,
    oracle_id = "5231a7d6-b6cf-4fa8-8da8-f0ecf023e054",
    scryfall_id = "48ab243e-d08d-4ece-9725-4bb5f67b1c92",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dwarven Landslide",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
