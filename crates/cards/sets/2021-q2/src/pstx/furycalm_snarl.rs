//! Furycalm Snarl — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Mountain or Plains card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {R} or {W}.
//! Set: MSC #247 — Marvel Super Heroes Commander | Scryfall ID: afc12892-4978-4442-ace5-c4dea5c0ee00 | Oracle ID: 651dea9c-2375-4e44-8e65-ba8e40f0c0ef
// IMPLEMENTED — reveal a Mountain or Plains card from hand or enter tapped; {R} or {W}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The basic land type, not `Filter::BASIC_LAND`: "a Mountain or Plains card" is
/// any card with the type, so a dual land or a shockland in hand
/// reveals for this one exactly as a basic does.
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
    Filter::HasSubtype(subtypes::land::PLAINS),
]);

card!(
    index = index::FURYCALM_SNARL,
    oracle_id = "651dea9c-2375-4e44-8e65-ba8e40f0c0ef",
    scryfall_id = "afc12892-4978-4442-ace5-c4dea5c0ee00",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Furycalm Snarl",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Red,
        ManaColor::White
    ])])],
);
