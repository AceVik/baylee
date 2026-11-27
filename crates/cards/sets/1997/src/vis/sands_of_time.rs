//! Sands of Time — {4} — Artifact
//! Oracle: Each player skips their untap step.
//! Oracle: At the beginning of each player's upkeep, that player simultaneously untaps each tapped artifact, creature, and land they control and taps each untapped artifact, creature, and land they control.
//! Set: VIS #153 — Visions | Scryfall ID: a782ee95-bde4-41f4-a947-b073cc4c1e7c | Oracle ID: c8f69b48-a3b2-4324-852d-11fad19d6432
// IMPLEMENTED — skips every player's untap step and toggles tapped state of the active player's artifacts, creatures, and lands on each upkeep.

use baylee_cards_dsl::prelude::*;

static ARTIFACT_CREATURE_OR_LAND: Filter =
    Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::LAND]);
static TARGET_PLAYER_ACL: Filter =
    Filter::And(&[ARTIFACT_CREATURE_OR_LAND, Filter::ControlledByActivePlayer]);

card!(
    index = index::SANDS_OF_TIME,
    oracle_id = "c8f69b48-a3b2-4324-852d-11fad19d6432",
    scryfall_id = "a782ee95-bde4-41f4-a947-b073cc4c1e7c",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sands of Time",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::SkipUntapStep {
                who: PlayerRel::EachPlayer,
            },
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::ToggleTapAll {
                filter: &TARGET_PLAYER_ACL,
            }],
        ),
    ],
);
