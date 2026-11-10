//! Festering Wound — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of your upkeep, you may put an infection counter on this Aura.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, this Aura deals X damage to that player, where X is the number of infection counters on this Aura.
//! Set: UDS #61 — Urza's Destiny | Scryfall ID: 927eed13-510b-4b06-811d-91a6a069cb8c | Oracle ID: 32638d03-c101-42f0-8a4b-8c6290257a28
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FESTERING_WOUND,
    oracle_id = "32638d03-c101-42f0-8a4b-8c6290257a28",
    scryfall_id = "927eed13-510b-4b06-811d-91a6a069cb8c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Festering Wound",
        mana_cost = mana!("{1}{B}"),
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
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounter {
                    kind: counters::INFECTION,
                    amount: Amount::Fixed(1)
                }]
            }]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached
            },
            &[Effect::DealDamage {
                amount: Amount::CountersOnSource(counters::INFECTION),
                target: TargetSpec::Player(PlayerRel::ActivePlayer)
            }]
        ),
    ],
);
