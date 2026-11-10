//! Port of Karfell — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {3}{U}{B}{B}, {T}, Sacrifice this land: Mill four cards, then return a creature card from your graveyard to the battlefield tapped. (To mill a card, put the top card of your library into your graveyard.)
//! Set: MKC #280 — Murders at Karlov Manor Commander | Scryfall ID: c1377ca1-aa54-4fed-b289-45309eb2f6b6 | Oracle ID: f500a8a4-6135-448c-9116-bd2695a72229

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PORT_OF_KARFELL,
    oracle_id = "f500a8a4-6135-448c-9116-bd2695a72229",
    scryfall_id = "c1377ca1-aa54-4fed-b289-45309eb2f6b6",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Port of Karfell",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        activated!(
            cost!("{3}{U}{B}{B}", TapSelf, SacrificeSelf),
            &[
                Effect::Mill {
                    amount: Amount::Fixed(4),
                    target: PlayerRel::You
                },
                Effect::ChooseFromYourGraveyard {
                    filter: &Filter::CREATURE,
                    destination: SearchDest::Battlefield,
                    tapped: true,
                    optional: false
                }
            ]
        ),
    ],
);
