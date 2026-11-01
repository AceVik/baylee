//! Endoskeleton — {2} — Artifact
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {2}, {T}: Target creature gets +0/+3 for as long as this artifact remains tapped.
//! Set: USG #294 — Urza's Saga | Scryfall ID: ce4630c3-bf8e-46ac-be68-f454c4ca1047 | Oracle ID: e9ef7f8c-21ef-45f0-af64-8ffa9f0089d1
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENDOSKELETON,
    oracle_id = "e9ef7f8c-21ef-45f0-af64-8ffa9f0089d1",
    scryfall_id = "ce4630c3-bf8e-46ac-be68-f454c4ca1047",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Endoskeleton",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::ModifyTargets {
                modifier: Modifier::ModifyPT(0, 3),
                duration: Duration::WhileSourceTapped {
                    control: false,
                    power_limited: false,
                },
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
