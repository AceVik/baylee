//! Foreboding Ruins — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Swamp or Mountain card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Set: MSC #244 — Marvel Super Heroes Commander | Scryfall ID: 76fdf6be-e13b-49ee-9d50-8625edb49951 | Oracle ID: 5c87e2fa-77f1-4978-b25f-f14d227301d1
// IMPLEMENTED — reveal a Swamp or Mountain card from hand or enter tapped; {B} or {R}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The basic land type, not `Filter::BASIC_LAND`: "a Swamp or Mountain card" is
/// any card with the type, so a dual land or a shockland in hand
/// reveals for this one exactly as a basic does.
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::SWAMP),
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
]);

card!(
    index = index::FOREBODING_RUINS,
    oracle_id = "5c87e2fa-77f1-4978-b25f-f14d227301d1",
    scryfall_id = "76fdf6be-e13b-49ee-9d50-8625edb49951",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Foreboding Ruins",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Black,
        ManaColor::Red
    ])]),],
);
