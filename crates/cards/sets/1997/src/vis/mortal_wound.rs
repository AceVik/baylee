//! Mortal Wound — {G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature is dealt damage, destroy it.
//! Set: VIS #113 — Visions | Scryfall ID: 808830ff-496a-41dc-8b64-334ddaca9435 | Oracle ID: f7b70077-5518-45f9-9833-01a4f8afac71
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MORTAL_WOUND,
    oracle_id = "f7b70077-5518-45f9-9833-01a4f8afac71",
    scryfall_id = "808830ff-496a-41dc-8b64-334ddaca9435",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mortal Wound",
        mana_cost = mana!("{G}"),
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
            Trigger::DealtDamage(&Filter::AttachedToBySource),
            &[Effect::DestroyAll {
                filter: &Filter::AttachedToBySource,
                no_regen: false,
            }],
        ),
    ],
);
