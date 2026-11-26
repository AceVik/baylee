//! Kill Switch — {3} — Artifact
//! Oracle: {2}, {T}: Tap all other artifacts. They don't untap during their controllers' untap steps for as long as this artifact remains tapped.
//! Set: NEM #133 — Nemesis | Scryfall ID: a94cfd63-7f2b-4c0a-8dcb-f22cf83e1e27 | Oracle ID: 0cd5eef6-6830-4c6d-8578-2726900c39a5
// IMPLEMENTED — {2}, {T}: taps all other artifacts and keeps those locked artifacts from untapping while Kill Switch remains tapped (CR 611.2c).

use baylee_cards_dsl::prelude::*;

static OTHER_ARTIFACT: Filter = Filter::And(&[Filter::ARTIFACT, Filter::Another]);

card!(
    index = index::KILL_SWITCH,
    oracle_id = "0cd5eef6-6830-4c6d-8578-2726900c39a5",
    scryfall_id = "a94cfd63-7f2b-4c0a-8dcb-f22cf83e1e27",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kill Switch",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[
            Effect::TapAll {
                filter: &OTHER_ARTIFACT,
            },
            Effect::ForEachMatching {
                filter: &OTHER_ARTIFACT,
                effects: &[Effect::AddTempModifier {
                    target: TargetSpec::EventObject,
                    modifier: Modifier::DoesNotUntap,
                    duration: Duration::WhileSourceTapped {
                        control: false,
                        power_limited: false,
                    },
                }],
            },
        ],
    )],
);
