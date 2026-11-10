//! Sand Squid — {3}{U} — Creature — Squid Beast
//! Oracle: Islandwalk (This creature can't be blocked as long as defending player controls an Island.)
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: Tap target creature. That creature doesn't untap during its controller's untap step for as long as this creature remains tapped.
//! Set: MMQ #96 — Mercadian Masques | Scryfall ID: 4efd7ce9-b920-409d-a4d2-a07fff280712 | Oracle ID: 40578245-2cb8-4a6a-aeff-2252719c2d1c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAND_SQUID,
    oracle_id = "40578245-2cb8-4a6a-aeff-2252719c2d1c",
    scryfall_id = "4efd7ce9-b920-409d-a4d2-a07fff280712",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sand Squid",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SQUID, subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
