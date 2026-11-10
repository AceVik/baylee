//! Disappear — {2}{U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {U}: Return enchanted creature and this Aura to their owners' hands.
//! Set: UDS #30 — Urza's Destiny | Scryfall ID: bdf280f4-74a1-4e6f-aec6-1852f04204e4 | Oracle ID: 039e2e39-8f7f-43f2-8448-de35ef7b8dc1
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_AND_THIS: Filter = Filter::Or(&[Filter::AttachedToBySource, Filter::This]);

card!(
    index = index::DISAPPEAR,
    oracle_id = "039e2e39-8f7f-43f2-8448-de35ef7b8dc1",
    scryfall_id = "bdf280f4-74a1-4e6f-aec6-1852f04204e4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Disappear",
        mana_cost = mana!("{2}{U}{U}"),
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
        activated!(
            cost!("{U}"),
            &[Effect::ReturnAllToHand {
                filter: &ENCHANTED_AND_THIS,
                opponents_only: false,
            }],
        ),
    ],
);
