//! Ancient Amphitheater — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Giant card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {R} or {W}.
//! Set: CM2 #232 — Commander Anthology Volume II | Scryfall ID: c68137c5-c2f9-4b0d-ac4b-12c519854166 | Oracle ID: 7211221d-d4d8-4bbe-9d2a-b82e005bfe8a
// IMPLEMENTED — reveal a Giant card from hand or enter tapped; {R} or {W}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The subtype alone, with no `Filter::CREATURE` beside it: the card
/// prints "a Giant card", and Magic prints tribal instants and sorceries
/// that carry a creature type without being creatures — the card type
/// would refuse a card this land accepts. (This pool prints none of
/// them, so that is the reason for the shape rather than a claim about
/// what is here.)
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::HasSubtype(subtypes::creature::GIANT);

card!(
    index = index::ANCIENT_AMPHITHEATER,
    oracle_id = "7211221d-d4d8-4bbe-9d2a-b82e005bfe8a",
    scryfall_id = "c68137c5-c2f9-4b0d-ac4b-12c519854166",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[face!(
        name = "Ancient Amphitheater",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Red,
        ManaColor::White,
    ])]),],
);
