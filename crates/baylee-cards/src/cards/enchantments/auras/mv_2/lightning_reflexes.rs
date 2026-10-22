//! Lightning Reflexes — {1}{R} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+0 and has first strike.
//! Set: DMR #128 — Dominaria Remastered | Scryfall ID: 9653f159-3f56-4446-bb6d-b4b4ee0cd61d | Oracle ID: 2b0c82f1-078a-4e8b-8e43-407f5c5d8549
// NOT SUPPORTED: sacrificing the Aura at the beginning of the next cleanup step if cast any time a sorcery couldn't have been cast.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::LIGHTNING_REFLEXES,
    oracle_id = "2b0c82f1-078a-4e8b-8e43-407f5c5d8549",
    scryfall_id = "9653f159-3f56-4446-bb6d-b4b4ee0cd61d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "has flash, enchants target creature, and gives enchanted creature +1/+0 and first strike; sacrificing it at the beginning of the next cleanup step if cast any time a sorcery couldn't have been cast is not supported",
    ),
    faces = &[face!(
        name = "Lightning Reflexes",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        keywords = KeywordSet::FLASH,
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(ENCHANTED_CREATURE, Modifier::ModifyPT(1, 0)),
        static_ability!(
            ENCHANTED_CREATURE,
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE)
        ),
    ],
);
