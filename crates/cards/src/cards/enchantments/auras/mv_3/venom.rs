//! Venom — {1}{G}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature blocks or becomes blocked by a non-Wall creature, destroy the other creature at end of combat.
//! Set: 5ED #336 — Fifth Edition | Scryfall ID: cbb503ef-31a4-492d-9c7d-26b72c36904b | Oracle ID: ee55ca31-73f3-4e9d-8373-0a44009a25bd

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const NON_WALL_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

card!(
    index = index::VENOM,
    oracle_id = "ee55ca31-73f3-4e9d-8373-0a44009a25bd",
    scryfall_id = "cbb503ef-31a4-492d-9c7d-26b72c36904b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Venom",
        mana_cost = mana!("{1}{G}{G}"),
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
            Trigger::AttachedBlocksOrBecomesBlockedBy(&NON_WALL_CREATURE),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[Effect::destroy(TargetSpec::EventObject)],
            }],
        ),
    ],
);
