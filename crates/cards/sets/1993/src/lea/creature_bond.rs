//! Creature Bond — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature dies, this Aura deals damage equal to that creature's toughness to the creature's controller.
//! Set: 4ED #66 — Fourth Edition | Scryfall ID: 717c5ee5-a033-4a58-bdcb-ef54e6c8b7a9 | Oracle ID: 70492e32-ba4d-4314-b016-892fb15f7a23
// IMPLEMENTED — enchant creature; its death deals its last toughness to its
// last controller. Behavioral coverage: engine/card_tests/enchantments/creature_bond.rs.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CREATURE_BOND,
    oracle_id = "70492e32-ba4d-4314-b016-892fb15f7a23",
    scryfall_id = "717c5ee5-a033-4a58-bdcb-ef54e6c8b7a9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Creature Bond",
        mana_cost = mana!("{1}{U}"),
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
            Trigger::Dies(&Filter::AttachedToBySource),
            &[Effect::DealDamage {
                amount: Amount::EventLastToughness,
                target: TargetSpec::Player(PlayerRel::ControllerOfEvent),
            }]
        ),
    ],
);
