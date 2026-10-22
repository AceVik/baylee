//! Mystic Veil — {1}{U} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has shroud. (It can't be the target of spells or abilities.)
//! Set: VIS #38 — Visions | Scryfall ID: 7ddb640d-5c54-4d0a-b8c2-e22fe04f96c2 | Oracle ID: 43c88fcd-aac9-4741-8fd6-31ac81c5bb6f
// PARTIAL: has flash, enchants creature, and grants shroud; sacrificing Mystic Veil at the beginning of the next cleanup step if cast at instant speed is not yet supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
card!(
    index = index::MYSTIC_VEIL,
    oracle_id = "43c88fcd-aac9-4741-8fd6-31ac81c5bb6f",
    scryfall_id = "7ddb640d-5c54-4d0a-b8c2-e22fe04f96c2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Partial(
        "has flash, enchants creature, and grants shroud; sacrificing Mystic Veil at the beginning of the next cleanup step if cast at instant speed is not yet supported"
    ),
    faces = &[face!(
        name = "Mystic Veil",
        mana_cost = mana!("{1}{U}"),
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
            Modifier::AddKeyword(KeywordSet::SHROUD),
        ),
    ],
);
