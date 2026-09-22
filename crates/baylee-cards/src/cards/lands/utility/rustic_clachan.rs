//! Rustic Clachan — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Kithkin card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {W}.
//! Oracle: Reinforce 1—{1}{W} ({1}{W}, Discard this card: Put a +1/+1 counter on target creature.)
//! Set: DDF #34 — Duel Decks: Elspeth vs. Tezzeret | Scryfall ID: f50160e9-0b25-4e81-814d-cdcda2fb325d | Oracle ID: cde68428-0033-4ede-92f1-ab91de0a41fb
// IMPLEMENTED — reveal a Kithkin card from hand or enter tapped; {W}, plus reinforce 1.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The subtype alone, with no `Filter::CREATURE` beside it: the card
/// prints "a Kithkin card", and Magic prints tribal instants and sorceries
/// that carry a creature type without being creatures — the card type
/// would refuse a card this land accepts. (This pool prints none of
/// them, so that is the reason for the shape rather than a claim about
/// what is here.)
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::HasSubtype(subtypes::creature::KITHKIN);

card!(
    index = index::RUSTIC_CLACHAN,
    oracle_id = "cde68428-0033-4ede-92f1-ab91de0a41fb",
    scryfall_id = "f50160e9-0b25-4e81-814d-cdcda2fb325d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Rustic Clachan",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{1}{W}", DiscardSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            zone = ActivationZone::Hand,
        ),
    ],
);
