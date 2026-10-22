//! Death Watch — {B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature dies, its controller loses life equal to its power and you gain life equal to its toughness.
//! Set: VIS #57 — Visions | Scryfall ID: 0e939d8f-6989-4884-989b-9cba566c9963 | Oracle ID: 6f5b6cba-3b57-4e6a-afa7-0cb62e5cc7d4
// IMPLEMENTED — enchants a creature; when enchanted creature dies, its controller loses life equal to its power and you gain life equal to its toughness.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEATH_WATCH,
    oracle_id = "6f5b6cba-3b57-4e6a-afa7-0cb62e5cc7d4",
    scryfall_id = "0e939d8f-6989-4884-989b-9cba566c9963",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Death Watch",
        mana_cost = mana!("{B}"),
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
            Trigger::Dies(&Filter::AttachedToBySource),
            &[
                Effect::LoseLife {
                    amount: Amount::EventPower,
                    target: PlayerRel::ControllerOfEvent,
                },
                Effect::GainLife {
                    amount: Amount::EventLastToughness,
                },
            ],
        ),
    ],
);
