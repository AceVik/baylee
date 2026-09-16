//! Undercity Sewers — (no cost) — Land — Island Swamp
//! Oracle: ({T}: Add {U} or {B}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #270 — Murders at Karlov Manor | Scryfall ID: 2b5801fb-2026-4f25-98bc-ebb2f99684b9 | Oracle ID: 08d80efc-9542-4ba2-824c-c8615d8d07f2
// PARTIAL — the {U}/{B} mana choice (two basic land types, so the card must
// print it itself) and the tapped entry are built; the enters-surveil clause
// has no variant and is dropped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UNDERCITY_SEWERS,
    oracle_id = "08d80efc-9542-4ba2-824c-c8615d8d07f2",
    scryfall_id = "2b5801fb-2026-4f25-98bc-ebb2f99684b9",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Partial("the enters trigger is surveil 1, which no Effect can say"),
    faces = &[face!(
        name = "Undercity Sewers",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::ISLAND, subtypes::land::SWAMP],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    // NOT SUPPORTED: "When this land enters, surveil 1." — there is no
    // surveil effect; Scry sends the card to the bottom and Mill cannot look.
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Blue,
        ManaColor::Black
    ])])],
);
