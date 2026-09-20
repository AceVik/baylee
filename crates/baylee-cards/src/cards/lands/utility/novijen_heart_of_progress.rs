//! Novijen, Heart of Progress — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {G}{U}, {T}: Put a +1/+1 counter on each creature that entered this turn.
//! Set: C21 #305 — Commander 2021 | Scryfall ID: 9a1e15e7-4ba6-41ad-b27b-aee2d037b6a7 | Oracle ID: b3b5137d-0225-4dba-9231-d235ab0f137c
// PARTIAL — {T}: Add {C} is built; the counter ability has no vocabulary.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NOVIJEN_HEART_OF_PROGRESS,
    oracle_id = "b3b5137d-0225-4dba-9231-d235ab0f137c",
    scryfall_id = "9a1e15e7-4ba6-41ad-b27b-aee2d037b6a7",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Novijen, Heart of Progress",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Partial("no Filter variant names a creature that entered this turn"),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{G}{U}, {T}: Put a +1/+1 counter on each creature
        // that entered this turn." — Effect::AddCounterFilter has the shape,
        // but the set it would sweep cannot be said: no Filter variant names
        // what entered this turn, so the ability is left off rather than
        // shipped as one that puts nothing anywhere.
    ],
);
