//! Viridian Lorebearers — {3}{G} — Creature — Elf Shaman
//! Oracle: {3}{G}, {T}: Target creature gets +X/+X until end of turn, where X is the number of artifacts your opponents control.
//! Set: 5DN #99 — Fifth Dawn | Scryfall ID: 85673b93-39b7-44e1-b27d-b716bde42f0b | Oracle ID: 080c0557-59ee-4c46-afd9-5baabf0dc3d3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIRIDIAN_LOREBEARERS,
    oracle_id = "080c0557-59ee-4c46-afd9-5baabf0dc3d3",
    scryfall_id = "85673b93-39b7-44e1-b27d-b716bde42f0b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Viridian Lorebearers",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SHAMAN],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
