//! Gilt-Leaf Palace — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Elf card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {B} or {G}.
//! Set: LRW #268 — Lorwyn | Scryfall ID: cc4bbbb5-4218-4b2a-9ca2-de4d5f5dda19 | Oracle ID: 85573a3d-2993-491a-8f8d-bbdb844fa84e
// IMPLEMENTED — reveal a Elf card from hand or enter tapped; {B} or {G}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The subtype alone, with no `Filter::CREATURE` beside it: the card
/// prints "a Elf card", and Magic prints tribal instants and sorceries
/// that carry a creature type without being creatures — the card type
/// would refuse a card this land accepts. (This pool prints none of
/// them, so that is the reason for the shape rather than a claim about
/// what is here.)
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::HasSubtype(subtypes::creature::ELF);

card!(
    index = index::GILT_LEAF_PALACE,
    oracle_id = "85573a3d-2993-491a-8f8d-bbdb844fa84e",
    scryfall_id = "cc4bbbb5-4218-4b2a-9ca2-de4d5f5dda19",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Gilt-Leaf Palace",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Black,
        ManaColor::Green,
    ])])],
);
