//! Vineglimmer Snarl — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Forest or Island card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {G} or {U}.
//! Set: SOC #420 — Secrets of Strixhaven Commander | Scryfall ID: 445bd162-b523-45f4-83d0-ffc702fc1fac | Oracle ID: 33f52df8-4b44-4422-8b0a-37fead9c894b
// IMPLEMENTED — reveal a Forest or Island card from hand or enter tapped; {G} or {U}.
// IMPLEMENTED — {T} for {G} or {U}; the entry clause stays undecided (Partial).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The basic land type, not `Filter::BASIC_LAND`: "a Forest or Island card" is
/// any card with the type, so a dual land or a shockland in hand
/// reveals for this one exactly as a basic does.
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::FOREST),
    Filter::HasSubtype(subtypes::land::ISLAND),
]);

card!(
    index = index::VINEGLIMMER_SNARL,
    oracle_id = "33f52df8-4b44-4422-8b0a-37fead9c894b",
    scryfall_id = "445bd162-b523-45f4-83d0-ffc702fc1fac",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Vineglimmer Snarl",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Green,
        ManaColor::Blue,
    ])])],
);
