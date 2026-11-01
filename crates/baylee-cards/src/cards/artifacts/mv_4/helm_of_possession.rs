//! Helm of Possession — {4} — Artifact
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {2}, {T}, Sacrifice a creature: Gain control of target creature for as long as you control this artifact and this artifact remains tapped.
//! Set: TMP #291 — Tempest | Scryfall ID: 79e16191-8dca-491b-b892-17696023d581 | Oracle ID: 78f22413-4e61-4191-9d30-7b5d6538446e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HELM_OF_POSSESSION,
    oracle_id = "78f22413-4e61-4191-9d30-7b5d6538446e",
    scryfall_id = "79e16191-8dca-491b-b892-17696023d581",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Helm of Possession",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            cost!("{2}", TapSelf, Sacrifice(&Filter::YOUR_CREATURE)),
            &[Effect::ModifyTargets {
                modifier: Modifier::GainControl,
                duration: Duration::WhileSourceTapped {
                    control: true,
                    power_limited: false,
                },
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
