//! Shackles — {2}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature doesn't untap during its controller's untap step.
//! Oracle: {W}: Return this Aura to its owner's hand.
//! Set: TPR #26 — Tempest Remastered | Scryfall ID: b5a92d31-d569-4520-b06a-0664fe050e64 | Oracle ID: 05861ff2-ec85-43d1-b641-c5d2da819c03
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHACKLES,
    oracle_id = "05861ff2-ec85-43d1-b641-c5d2da819c03",
    scryfall_id = "b5a92d31-d569-4520-b06a-0664fe050e64",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shackles",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::DoesNotUntap),
        activated!(cost!("{W}"), &[Effect::bounce(TargetSpec::ThisObject)]),
    ],
);
