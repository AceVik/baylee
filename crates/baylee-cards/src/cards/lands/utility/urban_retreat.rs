//! Urban Retreat — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}, {W}, or {U}.
//! Oracle: {2}, Return a tapped creature you control to its owner's hand: Put this card from your hand onto the battlefield. Activate only as a sorcery.
//! Set: SPM #187 — Marvel's Spider-Man | Scryfall ID: 2581f320-8238-413d-ab04-d5535da55630 | Oracle ID: 18290c4b-cff6-4ee5-a487-fe0d5706d3de

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URBAN_RETREAT,
    oracle_id = "18290c4b-cff6-4ee5-a487-fe0d5706d3de",
    scryfall_id = "2581f320-8238-413d-ab04-d5535da55630",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    faces = &[face!(
        name = "Urban Retreat",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[
            ManaColor::Green,
            ManaColor::White,
            ManaColor::Blue,
        ])]),
        activated!(
            cost!(
                "{2}",
                ReturnToHand(&Filter::And(&[Filter::YOUR_CREATURE, Filter::Tapped]))
            ),
            &[Effect::PutSourceFromHandOntoBattlefield],
            zone = ActivationZone::Hand,
            timing = ActivationTiming::SorcerySpeed
        ),
    ],
);
