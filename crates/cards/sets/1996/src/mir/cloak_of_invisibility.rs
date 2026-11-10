//! Cloak of Invisibility — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has phasing and can't be blocked except by Walls. (It phases in or out before its controller untaps during each of their untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Set: MIR #58 — Mirage | Scryfall ID: 3f6e21e6-fb3e-49c1-b5a0-499faf66d279 | Oracle ID: 68b06917-7f83-47d4-a466-4e6698cae56d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);
const NON_WALL_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

card!(
    index = index::CLOAK_OF_INVISIBILITY,
    oracle_id = "68b06917-7f83-47d4-a466-4e6698cae56d",
    scryfall_id = "3f6e21e6-fb3e-49c1-b5a0-499faf66d279",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cloak of Invisibility",
        mana_cost = mana!("{U}"),
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
            ENCHANTED_CREATURE,
            Modifier::AddKeyword(KeywordSet::PHASING)
        ),
        static_ability!(
            ENCHANTED_CREATURE,
            Modifier::CantBeBlockedBy(&NON_WALL_CREATURE),
        ),
    ],
);
