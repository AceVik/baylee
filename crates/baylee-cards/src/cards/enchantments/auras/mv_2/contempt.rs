//! Contempt — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature attacks, return it and this Aura to their owners' hands at end of combat.
//! Set: STH #27 — Stronghold | Scryfall ID: c1fa8e60-c32f-4586-b133-224f0aec3355 | Oracle ID: 811c8364-fafb-49c8-a5bf-7b6cdfceb475
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CONTEMPT,
    oracle_id = "811c8364-fafb-49c8-a5bf-7b6cdfceb475",
    scryfall_id = "c1fa8e60-c32f-4586-b133-224f0aec3355",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Contempt",
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
        triggered!(
            Trigger::Attacks(&Filter::AttachedToBySource),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[
                    Effect::bounce(TargetSpec::EventObject),
                    Effect::bounce(TargetSpec::ThisObject),
                ],
            }],
        ),
    ],
);
