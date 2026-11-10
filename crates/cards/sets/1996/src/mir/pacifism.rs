//! Pacifism — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't attack or block.
//! Set: MSC #575 — Marvel Super Heroes Commander | Scryfall ID: 433eed50-81af-4a59-b624-57b6f5f1bb0f | Oracle ID: 5f5e0b10-c8cf-450c-bfd3-bcb0528ec330
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::PACIFISM,
    oracle_id = "5f5e0b10-c8cf-450c-bfd3-bcb0528ec330",
    scryfall_id = "433eed50-81af-4a59-b624-57b6f5f1bb0f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pacifism",
        mana_cost = mana!("{1}{W}"),
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
        static_ability!(ENCHANTED_CREATURE, Modifier::CantAttack),
        static_ability!(ENCHANTED_CREATURE, Modifier::CantBlock(&Filter::Any)),
    ],
);
