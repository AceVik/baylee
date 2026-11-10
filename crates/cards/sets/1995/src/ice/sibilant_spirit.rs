//! Sibilant Spirit — {5}{U} — Creature — Spirit
//! Oracle: Flying
//! Oracle: Whenever this creature attacks, defending player may draw a card.
//! Set: ME2 #67 — Masters Edition II | Scryfall ID: ae32232f-3b75-45a2-bb25-1ff2e0f05bdc | Oracle ID: 9798d307-a44a-4f44-94c7-03e499b94962
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SIBILANT_SPIRIT,
    oracle_id = "9798d307-a44a-4f44-94c7-03e499b94962",
    scryfall_id = "ae32232f-3b75-45a2-bb25-1ff2e0f05bdc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sibilant Spirit",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(5),
        toughness = Some(6),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::MayDoFor {
            player: PlayerRel::DefendingPlayer,
            effects: &[Effect::DrawCardsFor {
                amount: Amount::Fixed(1),
                who: PlayerRel::DefendingPlayer,
            }],
        }],
    ),],
);
