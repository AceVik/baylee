//! Torture — {B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {1}{B}: Put a -1/-1 counter on enchanted creature.
//! Set: SHM #80 — Shadowmoor | Scryfall ID: d1fed16d-c800-4c0f-be88-9fddde2c018e | Oracle ID: 389b6d77-47ab-43a3-99d1-dcc762939cc4
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TORTURE,
    oracle_id = "389b6d77-47ab-43a3-99d1-dcc762939cc4",
    scryfall_id = "d1fed16d-c800-4c0f-be88-9fddde2c018e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Torture",
        mana_cost = mana!("{B}"),
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
        activated!(
            cost!("{1}{B}"),
            &[Effect::ChangeAttachedCounters {
                kind: CounterKind::M1M1,
                amount: Amount::Fixed(1),
                remove: false,
            }]
        ),
    ],
);
