//! Faerie Mastermind — {1}{U} — Creature — Faerie Rogue
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: Whenever an opponent draws their second card each turn, you draw a card.
//! Oracle: {3}{U}: Each player draws a card.
//! Set: SOC #114 — Secrets of Strixhaven Commander | Scryfall ID: bc9a7599-3c7d-4b83-bff3-e3d1f20f1b0f | Oracle ID: a984db23-40ea-428d-829f-e944267280f8
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FAERIE_MASTERMIND,
    oracle_id = "a984db23-40ea-428d-829f-e944267280f8",
    scryfall_id = "bc9a7599-3c7d-4b83-bff3-e3d1f20f1b0f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLASH.union(KeywordSet::FLYING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Faerie Mastermind",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FAERIE, subtypes::creature::ROGUE],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::NthCardDrawnInTurn {
                who: PlayerRel::Opponent,
                n: 2,
            },
            &[Effect::draw(1)],
        ),
        activated!(
            cost!("{3}{U}"),
            &[Effect::DrawCardsFor {
                amount: Amount::Fixed(1),
                who: PlayerRel::EachPlayer,
            }],
        ),
    ],
);
