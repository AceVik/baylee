//! Mob Mentality — {R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has trample.
//! Oracle: Whenever all non-Wall creatures you control attack, enchanted creature gets +X/+0 until end of turn, where X is the number of attacking creatures.
//! Set: VIS #88 — Visions | Scryfall ID: e428d56a-9445-4e86-b281-656e2d251e0b | Oracle ID: 90e8fb2c-6473-42c3-92f4-f46a744920fa
// PARTIAL: enchants creature and grants trample; the trigger when all non-Wall creatures you control attack is not yet supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Whenever all non-Wall creatures you control attack, enchanted creature gets +X/+0 until end of turn, where X is the number of attacking creatures.
card!(
    index = index::MOB_MENTALITY,
    oracle_id = "90e8fb2c-6473-42c3-92f4-f46a744920fa",
    scryfall_id = "e428d56a-9445-4e86-b281-656e2d251e0b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "enchants creature and grants trample; the trigger when all non-Wall creatures you control attack is not yet supported"
    ),
    faces = &[face!(
        name = "Mob Mentality",
        mana_cost = mana!("{R}"),
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
            Modifier::AddKeyword(KeywordSet::TRAMPLE),
        ),
    ],
);
