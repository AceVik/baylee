//! Varchild's War-Riders — {1}{R} — Creature — Human Warrior
//! Oracle: Cumulative upkeep—Have an opponent create a 1/1 red Survivor creature token. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Trample; rampage 1 (Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it beyond the first.)
//! Set: ME1 #110 — Masters Edition | Scryfall ID: e879b457-dffa-4b3b-a3cd-717a60c1e586 | Oracle ID: 258921f2-df26-4d8a-ad34-8f44d0158ae7
// NOT SUPPORTED: Cumulative upkeep has no optional opponent-token-creation payment variant, so upkeep puts an age counter on Varchild's War-Riders and has each opponent create that many 1/1 red Survivor creature tokens without offering to decline and sacrifice it.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VARCHILD_S_WAR_RIDERS,
    oracle_id = "258921f2-df26-4d8a-ad34-8f44d0158ae7",
    scryfall_id = "e879b457-dffa-4b3b-a3cd-717a60c1e586",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Partial(
        "Cumulative upkeep always pays by putting an age counter on Varchild's War-Riders and having each opponent create that many 1/1 red Survivor creature tokens rather than offering to decline and sacrifice it; Trample and Rampage 1 work as printed.",
    ),
    faces = &[face!(
        name = "Varchild's War-Riders",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::CreateTokensFor {
                    player: PlayerRel::EachOpponent,
                    token: &generated_tokens::SURVIVOR_1_1_RED,
                    amount: Amount::CountersOnSource(counters::AGE),
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        triggered!(
            Trigger::Rampage,
            &[Effect::PumpFilter {
                filter: &Filter::This,
                controlled_by: None,
                power: Amount::BlockersBeyondFirst,
                toughness: Amount::BlockersBeyondFirst,
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);
