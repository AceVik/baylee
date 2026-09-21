//! The Mycosynth Gardens — (no cost) — Land — Sphere
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {X}, {T}: This land becomes a copy of target nontoken artifact you control with mana value X.
//! Set: EOC #168 — Edge of Eternities Commander | Scryfall ID: 0a6f0408-6758-495f-9d6c-7686a1542fbd | Oracle ID: 03f5c566-825c-4c46-9c01-a2f9b1e70a13
// PARTIAL — {T}: Add {C} and {1}, {T}: Add one of any color are built; the {X}, {T} copy ability has no DSL variant.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_MYCOSYNTH_GARDENS,
    oracle_id = "03f5c566-825c-4c46-9c01-a2f9b1e70a13",
    scryfall_id = "0a6f0408-6758-495f-9d6c-7686a1542fbd",
    faces = &[face!(
        name = "The Mycosynth Gardens",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::SPHERE],
    ),],
    coverage = Coverage::Partial(
        "{X}, {T}: This land becomes a copy of target nontoken artifact — no DSL variant copies a target permanent"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        // NOT SUPPORTED: "{X}, {T}: This land becomes a copy of target nontoken artifact you control with mana value X."
    ],
);
