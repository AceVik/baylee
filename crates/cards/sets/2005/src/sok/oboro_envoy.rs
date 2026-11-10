//! Oboro Envoy — {3}{U} — Creature — Moonfolk Wizard
//! Oracle: Flying
//! Oracle: {2}, Return a land you control to its owner's hand: Target creature gets -X/-0 until end of turn, where X is the number of cards in your hand.
//! Set: SOK #49 — Saviors of Kamigawa | Scryfall ID: a0309dcb-6c13-4ffe-b44d-3b735c8277d2 | Oracle ID: be70c6e8-6f9f-49fb-ab40-c6ce0ec2077c
// IMPLEMENTED — flying; {2} plus returning a land you control buys -X/-0 on a
// target creature, X counted off the cards in your hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OBORO_ENVOY,
    oracle_id = "be70c6e8-6f9f-49fb-ab40-c6ce0ec2077c",
    scryfall_id = "a0309dcb-6c13-4ffe-b44d-3b735c8277d2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    faces = &[face!(
        name = "Oboro Envoy",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MOONFOLK, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{2}", ReturnToHand(&Filter::YOUR_LAND)),
        &[Effect::PumpTarget {
            power: Amount::Negated(&Amount::CountOf {
                filter: &Filter::Any,
                zone: ZoneSel::HandYou,
            }),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
