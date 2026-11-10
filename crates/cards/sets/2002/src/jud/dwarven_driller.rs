//! Dwarven Driller — {3}{R} — Creature — Dwarf
//! Oracle: {T}: Destroy target land unless its controller has this creature deal 2 damage to them.
//! Set: JUD #85 — Judgment | Scryfall ID: 69d815d3-7e33-4de9-aa36-ff5ffb893d73 | Oracle ID: 5967c3d3-2ef1-4496-b51c-23478fa77f09
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWARVEN_DRILLER,
    oracle_id = "5967c3d3-2ef1-4496-b51c-23478fa77f09",
    scryfall_id = "69d815d3-7e33-4de9-aa36-ff5ffb893d73",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dwarven Driller",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
