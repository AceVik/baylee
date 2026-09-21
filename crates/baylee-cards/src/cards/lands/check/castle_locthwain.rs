//! Castle Locthwain — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Swamp.
//! Oracle: {T}: Add {B}.
//! Oracle: {1}{B}{B}, {T}: Draw a card, then you lose life equal to the number of cards in your hand.
//! Set: CLB #884 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 19336e3a-2242-4a30-a563-32f2e4fc18e9 | Oracle ID: be811e70-aaaa-41f3-bf9e-5d3f9f719b49
// IMPLEMENTED — enters tapped unless you control a Swamp, {T} for {B}, and {1}{B}{B}, {T} draws a card then loses life equal to cards in hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SWAMP_YOU_CONTROL: Filter = f!(your Filter::HasSubtype(land::SWAMP));

card!(
    index = index::CASTLE_LOCTHWAIN,
    oracle_id = "be811e70-aaaa-41f3-bf9e-5d3f9f719b49",
    scryfall_id = "19336e3a-2242-4a30-a563-32f2e4fc18e9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Castle Locthwain",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&SWAMP_YOU_CONTROL)],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            cost!("{1}{B}{B}", TapSelf),
            &[
                Effect::draw(1),
                Effect::LoseLife {
                    amount: Amount::CountOf {
                        filter: &Filter::Any,
                        zone: ZoneSel::HandYou,
                    },
                    target: PlayerRel::You,
                },
            ],
        ),
    ],
);
