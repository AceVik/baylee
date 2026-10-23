//! Cloak of Mists — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked.
//! Set: GS1 #13 — Global Series Jiang Yanggu & Mu Yanling | Scryfall ID: b36f41fa-b676-475e-bc2a-7dce5f249989 | Oracle ID: 13358b1d-0a35-48ed-a7fb-70359fa3998f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CLOAK_OF_MISTS,
    oracle_id = "13358b1d-0a35-48ed-a7fb-70359fa3998f",
    scryfall_id = "b36f41fa-b676-475e-bc2a-7dce5f249989",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cloak of Mists",
        mana_cost = mana!("{1}{U}"),
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
            Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)
        ),
    ],
);
