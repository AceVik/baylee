//! Bravado — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+1 for each other creature you control.
//! Set: USG #177 — Urza's Saga | Scryfall ID: 019ba422-4837-400c-a913-b56d87b49e26 | Oracle ID: 69b5af09-b3e6-4abf-b62e-9199c4f43ba7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRAVADO,
    oracle_id = "69b5af09-b3e6-4abf-b62e-9199c4f43ba7",
    scryfall_id = "019ba422-4837-400c-a913-b56d87b49e26",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bravado",
        mana_cost = mana!("{1}{R}"),
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
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPTByCount(PtCount::YouControl(&Filter::And(&[
                Filter::CREATURE,
                Filter::Not(&Filter::AttachedToBySource)
            ])))
        ),
    ],
);
