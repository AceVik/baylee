//! Bloom Tender — {1}{G} — Creature — Elf Druid
//! Oracle: Vivid — {T}: For each color among permanents you control, add one mana of that color.
//! Set: ECL #166 — Lorwyn Eclipsed | Scryfall ID: ba86688d-18f0-4b5c-a797-42bf125a6c9f | Oracle ID: 0c23fefe-9891-4dd8-9bb1-eebdb3274e31
// IMPLEMENTED — {T} adds one mana per distinct color among the permanents you
// control: the count is `Amount::DistinctColorsAmong` over everything you
// control, spent through `Effect::mana_combination`, which makes one pick of
// a color per mana. "Vivid" is an ability word with no rule of its own, so it
// is stated nowhere but the Oracle line above.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLOOM_TENDER,
    oracle_id = "0c23fefe-9891-4dd8-9bb1-eebdb3274e31",
    scryfall_id = "ba86688d-18f0-4b5c-a797-42bf125a6c9f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bloom Tender",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[mana_ability!(&[Effect::mana_combination(
        ALL_MANA_COLORS,
        Amount::DistinctColorsAmong(&Filter::ControlledByYou),
    )])],
);
