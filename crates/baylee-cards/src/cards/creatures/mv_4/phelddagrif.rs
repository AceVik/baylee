//! Phelddagrif — {1}{G}{W}{U} — Legendary Creature — Phelddagrif
//! Oracle: {G}: Phelddagrif gains trample until end of turn. Target opponent creates a 1/1 green Hippo creature token.
//! Oracle: {W}: Phelddagrif gains flying until end of turn. Target opponent gains 2 life.
//! Oracle: {U}: Return Phelddagrif to its owner's hand. Target opponent may draw a card.
//! Set: ME1 #150 — Masters Edition | Scryfall ID: 493a72f0-2e35-4e06-a1b2-d42d432b9b2c | Oracle ID: d0e99535-0ea0-4aa5-99f5-6c5255b27c38
// IMPLEMENTED — {G} trample + opponent Hippo token, {W} flying + opponent gains 2 life, and {U} return self to hand + opponent may draw a card.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHELDDAGRIF,
    oracle_id = "d0e99535-0ea0-4aa5-99f5-6c5255b27c38",
    scryfall_id = "493a72f0-2e35-4e06-a1b2-d42d432b9b2c",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Phelddagrif",
        mana_cost = mana!("{1}{G}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::PHELDDAGRIF],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[
        activated!(
            cost!("{G}"),
            &[
                Effect::ModifySource {
                    modifier: Modifier::AddKeyword(KeywordSet::TRAMPLE),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateTokensFor {
                    player: PlayerRel::Chosen,
                    token: &generated_tokens::HIPPO_1_1_GREEN,
                    amount: Amount::Fixed(1),
                },
            ],
            target = Some(TargetSpec::AnyOpponent),
        ),
        activated!(
            cost!("{W}"),
            &[
                Effect::ModifySource {
                    modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::GainLifeFor {
                    amount: Amount::Fixed(2),
                    who: PlayerRel::Chosen,
                },
            ],
            target = Some(TargetSpec::AnyOpponent),
        ),
        activated!(
            cost!("{U}"),
            &[
                Effect::ReturnToHand {
                    target: TargetSpec::ThisObject,
                },
                Effect::MayDoFor {
                    player: PlayerRel::Chosen,
                    effects: &[Effect::DrawCardsFor {
                        amount: Amount::Fixed(1),
                        who: PlayerRel::Chosen,
                    }],
                },
            ],
            target = Some(TargetSpec::AnyOpponent),
        ),
    ],
);
