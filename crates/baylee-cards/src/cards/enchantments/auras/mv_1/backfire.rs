//! Backfire — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature deals damage to you, this Aura deals that much damage to that creature's controller.
//! Set: 4ED #62 — Fourth Edition | Scryfall ID: ad34094d-a7ec-4b04-a288-4d4f1a07fc6b | Oracle ID: e04fa58e-be7a-4c9c-98d0-48241742a41b

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BACKFIRE,
    oracle_id = "e04fa58e-be7a-4c9c-98d0-48241742a41b",
    scryfall_id = "ad34094d-a7ec-4b04-a288-4d4f1a07fc6b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Backfire",
        mana_cost = mana!("{U}"),
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
        triggered!(
            Trigger::DealsDamageToPlayer {
                source: &Filter::AttachedToBySource,
                player: PlayerRel::You
            },
            &[Effect::DealDamage {
                amount: Amount::EventAmount,
                target: TargetSpec::Player(PlayerRel::ControllerOfEvent)
            }]
        ),
    ],
);
