//! Auntie's Hovel — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Goblin card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Set: LRW #267 — Lorwyn | Scryfall ID: 098685c9-cd85-4279-a3b5-b495485bba35 | Oracle ID: 245469ff-72b6-4846-8a82-a1d29f4d09bb
// IMPLEMENTED — reveal a Goblin card from hand or enter tapped; {B} or {R}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The subtype alone, with no `Filter::CREATURE` beside it: the card
/// prints "a Goblin card", and Magic prints tribal instants and sorceries
/// that carry a creature type without being creatures — the card type
/// would refuse a card this land accepts. (This pool prints none of
/// them, so that is the reason for the shape rather than a claim about
/// what is here.)
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::HasSubtype(subtypes::creature::GOBLIN);

card!(
    index = index::AUNTIE_S_HOVEL,
    oracle_id = "245469ff-72b6-4846-8a82-a1d29f4d09bb",
    scryfall_id = "098685c9-cd85-4279-a3b5-b495485bba35",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Auntie's Hovel",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Black,
        ManaColor::Red,
    ])])],
);
