//! Lord of Tresserhorn — {1}{U}{B}{R} — Legendary Creature — Zombie
//! Oracle: When Lord of Tresserhorn enters, you lose 2 life, you sacrifice two creatures, and target opponent draws two cards.
//! Oracle: {B}: Regenerate Lord of Tresserhorn.
//! Set: ME1 #149 — Masters Edition | Scryfall ID: 1d9d8732-9ff2-42e4-bdfc-723cb6a76969 | Oracle ID: 283e7a23-ae25-48d1-abb5-cd021f66cc29
// IMPLEMENTED — ETB trigger losing 2 life, sacrificing two creatures, and having target opponent draw two cards, plus {B} regeneration.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LORD_OF_TRESSERHORN,
    oracle_id = "283e7a23-ae25-48d1-abb5-cd021f66cc29",
    scryfall_id = "1d9d8732-9ff2-42e4-bdfc-723cb6a76969",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lord of Tresserhorn",
        mana_cost = mana!("{1}{U}{B}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(10),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[
                Effect::LoseLife {
                    amount: Amount::Fixed(2),
                    target: PlayerRel::You,
                },
                Effect::SacrificeFilter {
                    who: PlayerRel::You,
                    filter: &Filter::CREATURE,
                },
                Effect::SacrificeFilter {
                    who: PlayerRel::You,
                    filter: &Filter::CREATURE,
                },
                Effect::DrawCardsFor {
                    amount: Amount::Fixed(2),
                    who: PlayerRel::Chosen,
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
        ),
        activated!(cost!("{B}"), &[Effect::regenerate(TargetSpec::ThisObject)]),
    ],
);
