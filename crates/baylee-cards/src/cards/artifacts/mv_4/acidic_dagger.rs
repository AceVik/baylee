//! Acidic Dagger — {4} — Artifact
//! Oracle: {4}, {T}: Whenever target creature deals combat damage to a non-Wall creature this turn, destroy that non-Wall creature. When the targeted creature leaves the battlefield this turn, sacrifice this artifact. Activate only before blockers are declared.
//! Set: MIR #291 — Mirage | Scryfall ID: 6dfdea79-2a7c-489d-8466-e6090c2a0919 | Oracle ID: bf034a20-e362-4f38-9bf9-857777c62598
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NON_WALL_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

card!(
    index = index::ACIDIC_DAGGER,
    oracle_id = "bf034a20-e362-4f38-9bf9-857777c62598",
    scryfall_id = "6dfdea79-2a7c-489d-8466-e6090c2a0919",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Acidic Dagger",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{4}", TapSelf),
        &[
            Effect::ModifyTargets {
                modifier: Modifier::GrantTriggered {
                    trigger: Trigger::DealsCombatDamageToCreature(&Filter::This),
                    effects: &[Effect::IfEventObjectMatches {
                        filter: &NON_WALL_CREATURE,
                        then: &[Effect::destroy(TargetSpec::EventObject)],
                    }],
                    target: None,
                },
                duration: Duration::UntilEndOfTurn,
            },
            Effect::WhenTargetLeavesBattlefieldThisTurn {
                effects: &[Effect::SacrificeSelf],
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        condition = Some(Condition::BeforeStep(StepKind::DeclareBlockers)),
    )],
);
