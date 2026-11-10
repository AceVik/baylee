//! Necroblossom Snarl — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Swamp or Forest card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {B} or {G}.
//! Set: SOC #389 — Secrets of Strixhaven Commander | Scryfall ID: 4355ba23-de6a-4f18-baf4-82ae47cc2965 | Oracle ID: 761ee6f9-b0fa-43c9-8d1f-9591ea18e52d
// IMPLEMENTED — reveal a Swamp or Forest card from hand or enter tapped; {B} or {G}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The basic land type, not `Filter::BASIC_LAND`: "a Swamp or Forest card" is
/// any card with the type, so a dual land or a shockland in hand
/// reveals for this one exactly as a basic does.
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::SWAMP),
    Filter::HasSubtype(subtypes::land::FOREST),
]);

card!(
    index = index::NECROBLOSSOM_SNARL,
    oracle_id = "761ee6f9-b0fa-43c9-8d1f-9591ea18e52d",
    scryfall_id = "4355ba23-de6a-4f18-baf4-82ae47cc2965",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Necroblossom Snarl",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Black,
        ManaColor::Green,
    ])])],
);
