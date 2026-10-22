//! Discordant Spirit — {2}{B}{R} — Creature — Spirit
//! Oracle: At the beginning of each end step, if it's an opponent's turn, put a +1/+1 counter on this creature for each 1 damage dealt to you this turn.
//! Oracle: At the beginning of your end step, remove all +1/+1 counters from this creature.
//! Set: MIR #261 — Mirage | Scryfall ID: be67b950-dfe3-4159-aa53-63df25d2a926 | Oracle ID: ce785306-eaf5-42b3-9840-884cd690ae0a
// IMPLEMENTED — end-step +1/+1 counters for damage dealt to you on an opponent's turn, removed at your end step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DISCORDANT_SPIRIT,
    oracle_id = "ce785306-eaf5-42b3-9840-884cd690ae0a",
    scryfall_id = "be67b950-dfe3-4159-aa53-63df25d2a926",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Discordant Spirit",
        mana_cost = mana!("{2}{B}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::DamageDealtToYouThisTurn,
            }],
            condition = Some(Condition::OpponentsTurn),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            &[Effect::RemoveAllCountersSelf {
                kind: CounterKind::P1P1,
            }],
        ),
    ],
);
