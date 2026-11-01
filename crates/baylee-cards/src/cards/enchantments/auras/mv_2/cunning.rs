//! Cunning — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +3/+3.
//! Oracle: When enchanted creature attacks or blocks, sacrifice this Aura at the beginning of the next cleanup step.
//! Set: EXO #28 — Exodus | Scryfall ID: 52f36bb8-5a97-4596-8ca3-707665770c76 | Oracle ID: d5e48041-5f21-48e6-bd0c-000f02dfbe8a
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CUNNING,
    oracle_id = "d5e48041-5f21-48e6-bd0c-000f02dfbe8a",
    scryfall_id = "52f36bb8-5a97-4596-8ca3-707665770c76",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cunning",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(3, 3)),
        triggered!(
            Trigger::Attacks(&Filter::AttachedToBySource),
            &[Effect::AtNextCleanup {
                effects: &[Effect::SacrificeSelf],
            }],
        ),
        triggered!(
            Trigger::Blocks(&Filter::AttachedToBySource),
            &[Effect::AtNextCleanup {
                effects: &[Effect::SacrificeSelf],
            }],
        ),
    ],
);
