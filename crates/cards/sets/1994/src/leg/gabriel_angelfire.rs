//! Gabriel Angelfire — {3}{G}{G}{W}{W} — Legendary Creature — Angel
//! Oracle: At the beginning of your upkeep, choose flying, first strike, trample, or rampage 3. Gabriel Angelfire gains that ability until your next upkeep. (Whenever a creature with rampage 3 becomes blocked, it gets +3/+3 until end of turn for each creature blocking it beyond the first.)
//! Set: ME3 #148 — Masters Edition III | Scryfall ID: 2e349074-1402-44cf-be19-33a661cff3b6 | Oracle ID: e8521fdf-0896-4747-8c66-ea3bb69eb876

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GABRIEL_ANGELFIRE,
    oracle_id = "e8521fdf-0896-4747-8c66-ea3bb69eb876",
    scryfall_id = "2e349074-1402-44cf-be19-33a661cff3b6",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::ChooseOne {
            choices: &[
                Effect::Sequence(&[Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::FLYING),
                    Duration::UntilYourNextUpkeep
                )]),
                Effect::Sequence(&[Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
                    Duration::UntilYourNextUpkeep
                )]),
                Effect::Sequence(&[Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::TRAMPLE),
                    Duration::UntilYourNextUpkeep
                )]),
                Effect::Sequence(&[Effect::continuous(
                    &Filter::This,
                    Modifier::GrantTriggered {
                        trigger: Trigger::Rampage,
                        effects: &[Effect::PumpFilter {
                            filter: &Filter::This,
                            controlled_by: None,
                            power: Amount::Times {
                                base: &Amount::BlockersBeyondFirst,
                                factor: 3
                            },
                            toughness: Amount::Times {
                                base: &Amount::BlockersBeyondFirst,
                                factor: 3
                            },
                            keywords: KeywordSet::EMPTY,
                            duration: Duration::UntilEndOfTurn,
                        }],
                        target: None,
                    },
                    Duration::UntilYourNextUpkeep
                )]),
            ]
        }]
    )],
    faces = &[face!(
        name = "Gabriel Angelfire",
        mana_cost = mana!("{3}{G}{G}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(4),
        toughness = Some(4),
    ),],
);
