//! Amber Prison — {4} — Artifact
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {4}, {T}: Tap target artifact, creature, or land. That permanent doesn't untap during its controller's untap step for as long as this artifact remains tapped.
//! Set: 6ED #272 — Classic Sixth Edition | Scryfall ID: db90a5e7-8238-442a-a6a4-78c59a6adb3d | Oracle ID: 1c69fdcf-ba87-480a-88df-70aa4ec9fff0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static ARTIFACT_CREATURE_OR_LAND: Filter =
    Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::LAND]);

card!(
    index = index::AMBER_PRISON,
    oracle_id = "1c69fdcf-ba87-480a-88df-70aa4ec9fff0",
    scryfall_id = "db90a5e7-8238-442a-a6a4-78c59a6adb3d",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Amber Prison",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            cost!("{4}", TapSelf),
            &[
                Effect::TapTarget,
                Effect::ModifyTargets {
                    modifier: Modifier::DoesNotUntap,
                    duration: Duration::WhileSourceTapped {
                        control: false,
                        power_limited: false,
                    },
                },
            ],
            target = Some(TargetSpec::Object(&ARTIFACT_CREATURE_OR_LAND)),
        ),
    ],
);
