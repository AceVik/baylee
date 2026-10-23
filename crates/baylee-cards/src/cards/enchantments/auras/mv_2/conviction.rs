//! Conviction — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+3.
//! Oracle: {W}: Return this Aura to its owner's hand.
//! Set: UMA #12 — Ultimate Masters | Scryfall ID: 94507660-cf60-4fd8-a796-7a6e6f28396d | Oracle ID: 4374b6b1-e691-4823-802d-8de8530aa461
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CONVICTION,
    oracle_id = "4374b6b1-e691-4823-802d-8de8530aa461",
    scryfall_id = "94507660-cf60-4fd8-a796-7a6e6f28396d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Conviction",
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
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPT(1, 3),
        ),
        activated!(cost!("{W}"), &[Effect::bounce(TargetSpec::ThisObject)]),
    ],
);
