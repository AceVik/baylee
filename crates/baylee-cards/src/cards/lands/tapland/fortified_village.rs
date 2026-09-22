//! Fortified Village — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Forest or Plains card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {G} or {W}.
//! Set: MSC #245 — Marvel Super Heroes Commander | Scryfall ID: 15f81ac6-a550-4782-a3a9-22d7fef1c206 | Oracle ID: 56f1a16a-9f41-41fb-b580-c200bca27cd6
// IMPLEMENTED — reveal a Forest or Plains card from hand or enter tapped; {G} or {W}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The basic land type, not `Filter::BASIC_LAND`: "a Forest or Plains card" is
/// any card with the type, so a dual land or a shockland in hand
/// reveals for this one exactly as a basic does.
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::FOREST),
    Filter::HasSubtype(subtypes::land::PLAINS),
]);

card!(
    index = index::FORTIFIED_VILLAGE,
    oracle_id = "56f1a16a-9f41-41fb-b580-c200bca27cd6",
    scryfall_id = "15f81ac6-a550-4782-a3a9-22d7fef1c206",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Fortified Village",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Green,
        ManaColor::White,
    ])])],
);
