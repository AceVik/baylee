//! Rapid Fire — {3}{W} — Instant
//! Oracle: Cast this spell only before blockers are declared.
//! Oracle: Target creature gains first strike until end of turn. If it doesn't have rampage, that creature gains rampage 2 until end of turn. (Whenever the creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)
//! Set: LEG #32 — Legends | Scryfall ID: e26e7c9c-e6de-47f4-8394-7e853408f84c | Oracle ID: 63c7248e-a7a2-4e32-b4d2-55dbc63e51e5

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAPID_FIRE,
    oracle_id = "63c7248e-a7a2-4e32-b4d2-55dbc63e51e5",
    scryfall_id = "e26e7c9c-e6de-47f4-8394-7e853408f84c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rapid Fire",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::FIRST_STRIKE,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::IfTargetMatches {
                filter: &Filter::Not(&Filter::HasRampage),
                then: &[Effect::continuous(
                    &Filter::This,
                    Modifier::GrantTriggered {
                        trigger: Trigger::Rampage,
                        effects: &[Effect::PumpFilter {
                            filter: &Filter::This,
                            controlled_by: None,
                            power: Amount::Times {
                                base: &Amount::BlockersBeyondFirst,
                                factor: 2
                            },
                            toughness: Amount::Times {
                                base: &Amount::BlockersBeyondFirst,
                                factor: 2
                            },
                            keywords: KeywordSet::EMPTY,
                            duration: Duration::UntilEndOfTurn,
                        }],
                        target: None,
                    },
                    Duration::UntilEndOfTurn
                )],
            }
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        condition = Some(Condition::BeforeStep(StepKind::DeclareBlockers))
    )],
);
