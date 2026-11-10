//! Wanderwine Hub — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Merfolk card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {W} or {U}.
//! Set: LRW #280 — Lorwyn | Scryfall ID: ccec69de-7203-4810-a8ec-8748705ee3a2 | Oracle ID: c3b46bd6-b3ef-452d-a916-995c44f1da07
// IMPLEMENTED — reveal a Merfolk card from hand or enter tapped; {W} or {U}.
// IMPLEMENTED — {T}: Add {W} or {U}; the enters-tapped clause is NOT
// SUPPORTED and the card says so below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The subtype alone, with no `Filter::CREATURE` beside it: the card
/// prints "a Merfolk card", and Magic prints tribal instants and sorceries
/// that carry a creature type without being creatures — the card type
/// would refuse a card this land accepts. (This pool prints none of
/// them, so that is the reason for the shape rather than a claim about
/// what is here.)
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::HasSubtype(subtypes::creature::MERFOLK);

card!(
    index = index::WANDERWINE_HUB,
    oracle_id = "c3b46bd6-b3ef-452d-a916-995c44f1da07",
    scryfall_id = "ccec69de-7203-4810-a8ec-8748705ee3a2",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Wanderwine Hub",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::White,
        ManaColor::Blue,
    ])])],
);
