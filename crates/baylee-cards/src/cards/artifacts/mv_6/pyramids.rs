//! Pyramids — {6} — Artifact
//! Oracle: {2}: Choose one —
//! Oracle: • Destroy target Aura attached to a land.
//! Oracle: • The next time target land would be destroyed this turn, remove all damage marked on it instead.
//! Set: ARN #67 — Arabian Nights | Scryfall ID: d2e9decf-47b7-44e0-b380-8055b6011021 | Oracle ID: da2f0d16-3cb4-492d-8535-52a31dbae95e
// IMPLEMENTED — modal Aura destruction or one-shot damage-clearing destruction replacement.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::enchantment::AURA;

const LAND_AURA: Filter =
    Filter::And(&[Filter::HasSubtype(AURA), Filter::AttachedTo(&Filter::LAND)]);

card!(
    index = index::PYRAMIDS,
    oracle_id = "da2f0d16-3cb4-492d-8535-52a31dbae95e",
    scryfall_id = "d2e9decf-47b7-44e0-b380-8055b6011021",
    faces = &[face!(
        name = "Pyramids",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT
    )],
    coverage = Coverage::Implemented,
    abilities = &[modal_activated!(
        cost!("{2}"),
        &[
            mode!(
                &[Effect::Destroy {
                    target: TargetSpec::Object(&LAND_AURA),
                    no_regen: false
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&LAND_AURA)))
            ),
            mode!(
                &[Effect::ModifyTargets {
                    modifier: Modifier::NextDestructionClearsDamage,
                    duration: Duration::UntilEndOfTurn,
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
            ),
        ]
    )],
);
