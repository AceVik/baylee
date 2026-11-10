//! Rainbow Efreet — {3}{U} — Creature — Efreet
//! Oracle: Flying
//! Oracle: {U}{U}: This creature phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before you untap during your next untap step.)
//! Set: VIS #41 — Visions | Scryfall ID: 1d6f03a6-3665-40e4-ae68-640913972770 | Oracle ID: 7c158405-d3ef-4c10-a504-7b061ef71a74
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAINBOW_EFREET,
    oracle_id = "7c158405-d3ef-4c10-a504-7b061ef71a74",
    scryfall_id = "1d6f03a6-3665-40e4-ae68-640913972770",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rainbow Efreet",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::EFREET],
        power = Some(3),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{U}{U}"),
        &[Effect::PhaseOut { target: None }]
    )],
);
