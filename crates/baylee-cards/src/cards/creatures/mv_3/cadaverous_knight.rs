//! Cadaverous Knight — {2}{B} — Creature — Zombie Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: {1}{B}{B}: Regenerate this creature.
//! Set: HOP #20 — Planechase | Scryfall ID: 6c658ea3-6fb3-4ce4-a6c5-3ed504102f15 | Oracle ID: 250a027f-2b3e-4eaf-899a-a9da8be20070
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CADAVEROUS_KNIGHT,
    oracle_id = "250a027f-2b3e-4eaf-899a-a9da8be20070",
    scryfall_id = "6c658ea3-6fb3-4ce4-a6c5-3ed504102f15",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cadaverous Knight",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    abilities = &[activated!(
        cost!("{1}{B}{B}"),
        &[Effect::regenerate(TargetSpec::ThisObject)],
    )],
);
