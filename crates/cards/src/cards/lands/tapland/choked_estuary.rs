//! Choked Estuary — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Island or Swamp card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {U} or {B}.
//! Set: MSC #229 — Marvel Super Heroes Commander | Scryfall ID: 0c37e84e-e7e0-4dfe-bf17-daeb2e434861 | Oracle ID: d473b507-8c33-4118-bc10-b0a268776074
// IMPLEMENTED — reveal a Island or Swamp card from hand or enter tapped; {U} or {B}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The basic land type, not `Filter::BASIC_LAND`: "a Island or Swamp card" is
/// any card with the type, so a dual land or a shockland in hand
/// reveals for this one exactly as a basic does.
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::ISLAND),
    Filter::HasSubtype(subtypes::land::SWAMP),
]);

card!(
    index = index::CHOKED_ESTUARY,
    oracle_id = "d473b507-8c33-4118-bc10-b0a268776074",
    scryfall_id = "0c37e84e-e7e0-4dfe-bf17-daeb2e434861",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Choked Estuary",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Blue,
        ManaColor::Black,
    ])])],
);
