//! Secluded Glen — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Faerie card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {U} or {B}.
//! Set: LRW #271 — Lorwyn | Scryfall ID: 9e4afa65-7933-4a64-b50f-a9a9f832b112 | Oracle ID: 09f52275-99e6-45e0-b2db-cafe26d5fb91
// IMPLEMENTED — reveal a Faerie card from hand or enter tapped; {U} or {B}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The subtype alone, with no `Filter::CREATURE` beside it: the card
/// prints "a Faerie card", and Magic prints tribal instants and sorceries
/// that carry a creature type without being creatures — the card type
/// would refuse a card this land accepts. (This pool prints none of
/// them, so that is the reason for the shape rather than a claim about
/// what is here.)
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::HasSubtype(subtypes::creature::FAERIE);

card!(
    index = index::SECLUDED_GLEN,
    oracle_id = "09f52275-99e6-45e0-b2db-cafe26d5fb91",
    scryfall_id = "9e4afa65-7933-4a64-b50f-a9a9f832b112",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Secluded Glen",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Blue,
        ManaColor::Black,
    ])])],
);
