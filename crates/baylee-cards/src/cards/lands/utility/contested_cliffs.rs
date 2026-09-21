//! Contested Cliffs — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {R}{G}, {T}: Target Beast creature you control fights target creature an opponent controls. (Each deals damage equal to its power to the other.)
//! Set: C13 #282 — Commander 2013 | Scryfall ID: 5796e5e4-32a0-4fe3-9912-80684aee489d | Oracle ID: b891a683-2ebc-4e9c-b402-5dd9c1b42b69
// PARTIAL — {T}: Add {C} is built; fight between two target creatures has no DSL variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONTESTED_CLIFFS,
    oracle_id = "b891a683-2ebc-4e9c-b402-5dd9c1b42b69",
    scryfall_id = "5796e5e4-32a0-4fe3-9912-80684aee489d",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(name = "Contested Cliffs", types = TypeSet::LAND,),],
    coverage = Coverage::Partial("Effect has no Fight variant to make two target creatures fight"),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{R}{G}, {T}: Target Beast creature you control fights target creature an opponent controls."
    ],
);
