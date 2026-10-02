//! Animate Artifact — {3}{U} — Enchantment — Aura
//! Oracle: Enchant artifact
//! Oracle: As long as enchanted artifact isn't a creature, it's an artifact creature with power and toughness each equal to its mana value.
//! Set: ME4 #38 — Masters Edition IV | Scryfall ID: 63aec27b-8cda-46c2-9f4f-9ebe98dffe2e | Oracle ID: 2dd7a4dc-902a-4e85-8a3b-c96a898fba86
// IMPLEMENTED — noncreature artifact animation, with the same affected object
// carried from the type layer into mana-value base power/toughness.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANIMATE_ARTIFACT,
    oracle_id = "2dd7a4dc-902a-4e85-8a3b-c96a898fba86",
    scryfall_id = "63aec27b-8cda-46c2-9f4f-9ebe98dffe2e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Animate Artifact",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::ARTIFACT)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT)))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AnimateNoncreatureArtifact
        ),
    ],
);

// Behavior: engine/card_tests/enchantments/animate_artifact.rs.
