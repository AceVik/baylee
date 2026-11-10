//! Flamekin Village — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Elemental card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: {R}, {T}: Target creature gains haste until end of turn.
//! Set: ECC #149 — Lorwyn Eclipsed Commander | Scryfall ID: df0affe7-92d8-422f-833d-74089626b829 | Oracle ID: 34a1eb04-08f6-49d8-a1d1-b987a76bd8b1
// IMPLEMENTED — reveal a Elemental card from hand or enter tapped; {R}, plus the haste ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The subtype alone, with no `Filter::CREATURE` beside it: the card
/// prints "a Elemental card", and Magic prints tribal instants and sorceries
/// that carry a creature type without being creatures — the card type
/// would refuse a card this land accepts. (This pool prints none of
/// them, so that is the reason for the shape rather than a claim about
/// what is here.)
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::HasSubtype(subtypes::creature::ELEMENTAL);

card!(
    index = index::FLAMEKIN_VILLAGE,
    oracle_id = "34a1eb04-08f6-49d8-a1d1-b987a76bd8b1",
    scryfall_id = "df0affe7-92d8-422f-833d-74089626b829",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Flamekin Village",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{R}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::HASTE,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
