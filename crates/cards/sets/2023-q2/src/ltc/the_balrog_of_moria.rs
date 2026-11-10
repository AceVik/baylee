//! The Balrog of Moria — {4}{B}{B}{R} — Legendary Creature — Avatar Demon
//! Oracle: Trample, haste
//! Oracle: When The Balrog of Moria dies, you may exile it. When you do, for each opponent, exile up to one target creature that player controls.
//! Oracle: Cycling {3}{R} ({3}{R}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, create two Treasure tokens.
//! Set: LTC #46 — Tales of Middle-earth Commander | Scryfall ID: 2a3fcfdc-f2cf-42d8-8bb4-7308bc12746e | Oracle ID: d73191d8-6f94-4fba-acd2-2d0490e3ac00

use crate::tokens::TREASURE;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_BALROG_OF_MORIA,
    oracle_id = "d73191d8-6f94-4fba-acd2-2d0490e3ac00",
    scryfall_id = "2a3fcfdc-f2cf-42d8-8bb4-7308bc12746e",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "The Balrog of Moria",
        mana_cost = mana!("{4}{B}{B}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::AVATAR, subtypes::creature::DEMON],
        power = Some(8),
        toughness = Some(8),
        keywords = KeywordSet::TRAMPLE.union(KeywordSet::HASTE),
    ),],
    abilities = &[
        // "You may exile it. When you do, for each opponent, exile up to one
        // target creature that player controls": the targets are chosen
        // after the exile, as the reflexive trigger goes on the stack
        // (CR 603.12), one opponent at a time.
        triggered!(
            Trigger::Dies(&Filter::This),
            &[
                Effect::MayDo {
                    effects: &[Effect::ExileSource],
                },
                Effect::Reflexive {
                    when: ReflexiveEvent::ExiledThis,
                    effects: &[Effect::Exile {
                        target: TargetSpec::ObjectOfEachOpponent(&Filter::CREATURE),
                    }],
                    target: Some(TargetSpec::ObjectOfEachOpponent(&Filter::CREATURE)),
                },
            ]
        ),
        // Cycling {3}{R} (CR 702.29a).
        activated!(
            cost!("{3}{R}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand
        ),
        triggered!(
            Trigger::CycledThis,
            &[Effect::CreateTokenN {
                token: &TREASURE,
                amount: Amount::Fixed(2),
            }]
        ),
    ],
);
