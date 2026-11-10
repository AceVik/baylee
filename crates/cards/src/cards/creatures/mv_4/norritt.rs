//! Norritt — {3}{B} — Creature — Imp
//! Oracle: {T}: Untap target blue creature.
//! Oracle: {T}: Choose target non-Wall creature the active player has controlled continuously since the beginning of the turn. That creature attacks this turn if able. Destroy it at the beginning of the next end step if it didn't attack this turn. Activate only before attackers are declared.
//! Set: ICE #155 — Ice Age | Scryfall ID: 35abefe6-c39b-4fe5-b2e3-d213f0c4f447 | Oracle ID: 4318c704-888f-4d85-b5f7-6d9abb14e60c
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLUE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::of(Color::Blue)),
]);

static GOADABLE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
    Filter::ControlledByActivePlayer,
    Filter::ControlledSinceTurnBegan,
]);

card!(
    index = index::NORRITT,
    oracle_id = "4318c704-888f-4d85-b5f7-6d9abb14e60c",
    scryfall_id = "35abefe6-c39b-4fe5-b2e3-d213f0c4f447",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Norritt",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::IMP],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            Cost::TAP,
            &[Effect::UntapTarget],
            targets = Some(TargetReq::one(TargetSpec::Object(&BLUE_CREATURE)))
        ),
        activated!(
            Cost::TAP,
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AttacksEachCombat,
                    Duration::UntilEndOfTurn,
                ),
                Effect::AtNextEndStep {
                    effects: &[Effect::IfEventObjectMatches {
                        filter: &Filter::Not(&Filter::AttackedThisTurn),
                        then: &[Effect::destroy(TargetSpec::EventObject)],
                    }],
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(&GOADABLE))),
            condition = Some(Condition::BeforeStep(StepKind::DeclareAttackers))
        ),
    ],
);
