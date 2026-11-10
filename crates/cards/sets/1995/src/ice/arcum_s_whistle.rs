//! Arcum's Whistle — {3} — Artifact
//! Oracle: {3}, {T}: Choose target non-Wall creature the active player has controlled continuously since the beginning of the turn. That player may pay {X}, where X is that creature's mana value. If they don't pay, the creature attacks this turn if able, and at the beginning of the next end step, destroy it if it didn't attack this turn. Activate only before attackers are declared.
//! Set: ICE #311 — Ice Age | Scryfall ID: 73c07c87-0e44-4a5a-92b7-728350cd02de | Oracle ID: de691eed-d90b-48f9-bae4-f982767f2bfd

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TARGET_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
    Filter::ControlledByActivePlayer,
    Filter::ControlledSinceTurnBegan,
]);

card!(
    index = index::ARCUM_S_WHISTLE,
    oracle_id = "de691eed-d90b-48f9-bae4-f982767f2bfd",
    scryfall_id = "73c07c87-0e44-4a5a-92b7-728350cd02de",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Arcum's Whistle",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[Effect::PlayerMayPayThenElse {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::TargetCmc,
            paid: &[],
            unpaid: &[
                Effect::ModifyTargets {
                    modifier: Modifier::AttacksEachCombat,
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::AtNextEndStep {
                    effects: &[Effect::IfEventObjectMatches {
                        filter: &Filter::Not(&Filter::AttackedThisTurn),
                        then: &[Effect::destroy(TargetSpec::EventObject)],
                    }],
                },
            ],
        }],
        target = Some(TargetSpec::Object(&TARGET_CREATURE)),
        condition = Some(Condition::BeforeStep(StepKind::DeclareAttackers))
    )],
);
