//! Faeburrow Elder — {1}{G}{W} — Creature — Treefolk Druid
//! Oracle: Vigilance
//! Oracle: This creature gets +1/+1 for each color among permanents you control.
//! Oracle: {T}: For each color among permanents you control, add one mana of that color.
//! Set: ECC #53 — Lorwyn Eclipsed Commander | Scryfall ID: 145dcac4-2c53-4ae9-9ede-24dc07474b01 | Oracle ID: 70a6f08e-854d-4e2f-9d8c-c45ec3231157

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FAEBURROW_ELDER,
    oracle_id = "70a6f08e-854d-4e2f-9d8c-c45ec3231157",
    scryfall_id = "145dcac4-2c53-4ae9-9ede-24dc07474b01",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    keywords = KeywordSet::VIGILANCE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Faeburrow Elder",
        mana_cost = mana!("{1}{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK, subtypes::creature::DRUID],
        power = Some(0),
        toughness = Some(0),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::ModifyPTByCount(PtCount::DistinctColorsAmong(&Filter::ControlledByYou))
        ),
        mana_ability!(&[Effect::ManaForColorsAmong(&Filter::ControlledByYou)]),
    ],
);
