//! Moonring Island — (no cost) — Land — Island
//! Oracle: ({T}: Add {U}.)
//! Oracle: This land enters tapped.
//! Oracle: {U}, {T}: Look at the top card of target player's library. Activate only if you control two or more blue permanents.
//! Set: SHM #276 — Shadowmoor | Scryfall ID: 64b36993-666c-40d0-b61a-1d162bd06dcc | Oracle ID: cf620c66-7db1-4db8-ae56-ee4bc2f77d74
// PARTIAL — enters tapped and taps for {U}; the second activated ability is
// left off, see the NOT SUPPORTED note below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOONRING_ISLAND,
    oracle_id = "cf620c66-7db1-4db8-ae56-ee4bc2f77d74",
    scryfall_id = "64b36993-666c-40d0-b61a-1d162bd06dcc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Moonring Island",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::ISLAND],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "no Effect variant for \"look at the top card of target player's library\""
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
);

// NOT SUPPORTED: "{U}, {T}: Look at the top card of target player's library.
// Activate only if you control two or more blue permanents." — the activation
// condition is expressible (Condition::ControlCount over a blue-permanent
// filter), the effect is not: no Effect reads the top card of another
// player's library without moving it (LookAtTopPick and ReorderTopLibrary are
// both your own library, and both move the cards). The ability comes off
// rather than being written as one that resolves and does nothing.
