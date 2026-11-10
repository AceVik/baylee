//! Osai Vultures — {1}{W} — Creature — Bird
//! Oracle: Flying
//! Oracle: At the beginning of each end step, if a creature died this turn, put a carrion counter on this creature.
//! Oracle: Remove two carrion counters from this creature: This creature gets +1/+1 until end of turn.
//! Set: ME4 #21 — Masters Edition IV | Scryfall ID: 2c665c02-4010-4bc6-85f0-bc03bef38ce9 | Oracle ID: 65b7a7ca-0e39-4612-9bcd-743bf903544a

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OSAI_VULTURES,
    oracle_id = "65b7a7ca-0e39-4612-9bcd-743bf903544a",
    scryfall_id = "2c665c02-4010-4bc6-85f0-bc03bef38ce9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Osai Vultures",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer
            },
            &[Effect::AddCounterSource {
                kind: counters::CARRION,
                amount: Amount::Fixed(1)
            }],
            condition = Some(Condition::CreaturesDiedThisTurnAtLeast(1))
        ),
        activated!(
            cost!(RemoveCounterSelf {
                kind: counters::CARRION,
                n: 2
            }),
            &[Effect::ModifySource {
                modifier: Modifier::ModifyPT(1, 1),
                duration: Duration::UntilEndOfTurn
            }]
        ),
    ],
);
