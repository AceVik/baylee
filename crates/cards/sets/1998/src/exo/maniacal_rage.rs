//! Maniacal Rage — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+2 and can't block.
//! Set: M20 #149 — Core Set 2020 | Scryfall ID: 29a51b9b-881f-4f02-b49e-68fcca234161 | Oracle ID: b5dbb212-0399-475b-a831-921f7c76cdc4
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MANIACAL_RAGE,
    oracle_id = "b5dbb212-0399-475b-a831-921f7c76cdc4",
    scryfall_id = "29a51b9b-881f-4f02-b49e-68fcca234161",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Maniacal Rage",
        mana_cost = mana!("{1}{R}"),
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
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 2)),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::CantBlock(&Filter::Any)
        ),
    ],
);
