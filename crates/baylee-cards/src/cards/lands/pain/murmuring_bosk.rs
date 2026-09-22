//! Murmuring Bosk — (no cost) — Land — Forest
//! Oracle: ({T}: Add {G}.)
//! Oracle: As this land enters, you may reveal a Treefolk card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {W} or {B}. This land deals 1 damage to you.
//! Set: DMC #220 — Dominaria United Commander | Scryfall ID: 5aca73a9-e90d-48c6-bdd9-9a3f4f552de3 | Oracle ID: 42b9d383-3fe2-4fc8-ab86-f80a288d502b
// IMPLEMENTED — reveal a Treefolk card from hand or enter tapped; {G} free, or {W}/{B} for a damage.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The subtype alone, with no `Filter::CREATURE` beside it: the card
/// prints "a Treefolk card", and Magic prints tribal instants and sorceries
/// that carry a creature type without being creatures — the card type
/// would refuse a card this land accepts. (This pool prints none of
/// them, so that is the reason for the shape rather than a claim about
/// what is here.)
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::HasSubtype(subtypes::creature::TREEFOLK);

card!(
    index = index::MURMURING_BOSK,
    oracle_id = "42b9d383-3fe2-4fc8-ab86-f80a288d502b",
    scryfall_id = "5aca73a9-e90d-48c6-bdd9-9a3f4f552de3",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Murmuring Bosk",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
        subtypes = &[subtypes::land::FOREST],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        mana_ability!(&[
            Effect::mana_choice(&[ManaColor::White, ManaColor::Black]),
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ]),
    ],
);
