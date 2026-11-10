//! Anti-Magic Aura — {2}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be the target of spells and can't be enchanted by other Auras.
//! Set: 5ED #72 — Fifth Edition | Scryfall ID: 54642f0e-2d5f-49eb-8181-054c84038072 | Oracle ID: 6f78c1e2-e38f-431b-8864-8aad982e9912

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANTI_MAGIC_AURA,
    oracle_id = "6f78c1e2-e38f-431b-8864-8aad982e9912",
    scryfall_id = "54642f0e-2d5f-49eb-8181-054c84038072",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Anti-Magic Aura",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::CantBeEnchantedExceptSource
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::CantBeTargetedBy(&Filter::InZone(ZoneRef::Stack))
        ),
    ],
);
