//! Thundering Falls — (no cost) — Land — Island Mountain
//! Oracle: ({T}: Add {U} or {R}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #269 — Murders at Karlov Manor | Scryfall ID: 17260fff-b239-4af4-9306-3236ae3fa5a5 | Oracle ID: d2bcff58-7a8a-46ef-b6b3-39501d4c8e6e
// PARTIAL — the {U}/{R} mana choice (two basic land types, so the card must
// print it itself) and the tapped entry are built; the enters-surveil clause
// has no variant and is dropped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THUNDERING_FALLS,
    oracle_id = "d2bcff58-7a8a-46ef-b6b3-39501d4c8e6e",
    scryfall_id = "17260fff-b239-4af4-9306-3236ae3fa5a5",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    coverage = Coverage::Partial("the enters trigger is surveil 1, which no Effect can say"),
    faces = &[face!(
        name = "Thundering Falls",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::ISLAND, subtypes::land::MOUNTAIN],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    // NOT SUPPORTED: "When this land enters, surveil 1." — there is no
    // surveil effect; Scry sends the card to the bottom and Mill cannot look.
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Blue,
        ManaColor::Red
    ])])],
);
