//! Curse Artifact — {2}{B}{B} — Enchantment — Aura
//! Oracle: Enchant artifact
//! Oracle: At the beginning of the upkeep of enchanted artifact's controller, this Aura deals 2 damage to that player unless they sacrifice that artifact.
//! Set: DRK #43 — The Dark | Scryfall ID: 9fc0d070-8a42-4d5e-8f2b-ceb59147de6f | Oracle ID: f08ae501-8b96-40ee-aa7b-c25293ddfa88

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CURSE_ARTIFACT,
    oracle_id = "f08ae501-8b96-40ee-aa7b-c25293ddfa88",
    scryfall_id = "9fc0d070-8a42-4d5e-8f2b-ceb59147de6f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Curse Artifact",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::ARTIFACT),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached,
            },
            &[Effect::PlayerMayPayCostOr {
                player: PlayerRel::ControllerOfAttached,
                cost: &CostPart::Sacrifice(&Filter::AttachedToBySource),
                effect: &Effect::DealDamage {
                    amount: Amount::Fixed(2),
                    target: TargetSpec::Player(PlayerRel::ControllerOfAttached),
                },
            }],
        ),
    ],
);
