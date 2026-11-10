//! Varchild's War-Riders — {1}{R} — Creature — Human Warrior
//! Oracle: Cumulative upkeep—Have an opponent create a 1/1 red Survivor creature token. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Trample; rampage 1 (Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it beyond the first.)
//! Set: ME1 #110 — Masters Edition | Scryfall ID: e879b457-dffa-4b3b-a3cd-717a60c1e586 | Oracle ID: 258921f2-df26-4d8a-ad34-8f44d0158ae7
// PARTIAL — Trample and rampage 1 as printed; the cumulative upkeep always pays (an age counter, then each opponent creates a 1/1 red Survivor per age counter), so its controller can never decline and sacrifice the creature.
// NOT SUPPORTED: "Cumulative upkeep—Have an opponent create a 1/1 red Survivor creature token" as a payment the controller may decline (then sacrifice it): no Effect pays with an effect and runs the sacrifice on a refusal. With more than one opponent every opponent creates the tokens, not one opponent of the controller's choice.

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
        "The cumulative upkeep always pays (an age counter, then each opponent creates a 1/1 red Survivor token per age counter), so the controller cannot decline it and sacrifice the creature, and with several opponents each of them creates the tokens, not one chosen opponent; Trample and rampage 1 work as printed.",
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
