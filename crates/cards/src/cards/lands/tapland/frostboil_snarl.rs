//! Frostboil Snarl — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Island or Mountain card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {U} or {R}.
//! Set: MSC #246 — Marvel Super Heroes Commander | Scryfall ID: ae705a26-5371-4236-904c-fe3e58b4721d | Oracle ID: 7137aae6-260d-41de-8b4e-42a8cf752697
// IMPLEMENTED — reveal a Island or Mountain card from hand or enter tapped; {U} or {R}.
// IMPLEMENTED — the mana ability; the reveal-as-it-enters clause is not expressible.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The basic land type, not `Filter::BASIC_LAND`: "a Island or Mountain card" is
/// any card with the type, so a dual land or a shockland in hand
/// reveals for this one exactly as a basic does.
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::ISLAND),
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
]);

card!(
    index = index::FROSTBOIL_SNARL,
    oracle_id = "7137aae6-260d-41de-8b4e-42a8cf752697",
    scryfall_id = "ae705a26-5371-4236-904c-fe3e58b4721d",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Frostboil Snarl",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Blue,
        ManaColor::Red,
    ])])],
);
