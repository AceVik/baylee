//! Halimar Depths — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, look at the top three cards of your library, then put them back in any order.
//! Oracle: {T}: Add {U}.
//! Set: DSC #282 — Duskmourn: House of Horror Commander | Scryfall ID: 63eb18e0-c723-4de4-8498-c5362c75b2b4 | Oracle ID: 42d121a2-5266-483a-ab16-e0a8073cd6a3
// IMPLEMENTED — enters tapped; ETB rearranges the top three cards of the library; {T} for {U}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HALIMAR_DEPTHS,
    oracle_id = "42d121a2-5266-483a-ab16-e0a8073cd6a3",
    scryfall_id = "63eb18e0-c723-4de4-8498-c5362c75b2b4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Halimar Depths",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        triggered!(Trigger::ETB, &[Effect::ReorderTopLibrary { count: 3 }]),
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
    ],
);
