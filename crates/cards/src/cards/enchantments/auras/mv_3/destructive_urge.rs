//! Destructive Urge — {1}{R}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature deals combat damage to a player, that player sacrifices a land of their choice.
//! Set: USG #180 — Urza's Saga | Scryfall ID: 479645b3-54af-4fbf-928e-2224540fe892 | Oracle ID: 5ab62b09-1135-4d22-866b-3a0d64228bf9
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DESTRUCTIVE_URGE,
    oracle_id = "5ab62b09-1135-4d22-866b-3a0d64228bf9",
    scryfall_id = "479645b3-54af-4fbf-928e-2224540fe892",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Destructive Urge",
        mana_cost = mana!("{1}{R}{R}"),
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
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::AttachedToBySource),
            &[Effect::SacrificeFilter {
                who: PlayerRel::DamagedPlayer,
                filter: &Filter::LAND,
            }],
        ),
    ],
);
