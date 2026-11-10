//! Thelon's Chant — {1}{G}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {G}.
//! Oracle: Whenever a player puts a Swamp onto the battlefield, this enchantment deals 3 damage to that player unless the player puts a -1/-1 counter on a creature they control.
//! Set: FEM #76 — Fallen Empires | Scryfall ID: 9d970195-0a09-4cb4-a2c0-c16fcab5c859 | Oracle ID: ff4ccf8d-7d3d-4a44-a9ab-6bc88e90d6a7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const SWAMP: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::SWAMP)]);

card!(
    index = index::THELON_S_CHANT,
    oracle_id = "ff4ccf8d-7d3d-4a44-a9ab-6bc88e90d6a7",
    scryfall_id = "9d970195-0a09-4cb4-a2c0-c16fcab5c859",
    coverage = Coverage::Implemented,
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Thelon's Chant",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{G}"),
                effect: &Effect::SacrificeSelf,
            }],
        ),
        triggered!(
            Trigger::EntersBattlefield(&SWAMP),
            &[Effect::PlayerMayPayCostOr {
                player: PlayerRel::ControllerOfEvent,
                cost: &CostPart::PutCounterOnOther {
                    filter: &Filter::CREATURE,
                    kind: CounterKind::M1M1,
                    n: 1,
                },
                effect: &Effect::DealDamage {
                    amount: Amount::Fixed(3),
                    target: TargetSpec::Player(PlayerRel::ControllerOfEvent),
                },
            }],
        ),
    ],
);
