//! Tourach's Chant — {1}{B}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {B}.
//! Oracle: Whenever a player puts a Forest onto the battlefield, this enchantment deals 3 damage to that player unless they put a -1/-1 counter on a creature they control.
//! Set: FEM #47 — Fallen Empires | Scryfall ID: 06883fd2-eccd-47c6-8c34-10d95e923685 | Oracle ID: bdf4e225-23b5-47d9-9936-ddcec8aa1ca4
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const FOREST: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::FOREST)]);

card!(
    index = index::TOURACH_S_CHANT,
    oracle_id = "bdf4e225-23b5-47d9-9936-ddcec8aa1ca4",
    scryfall_id = "06883fd2-eccd-47c6-8c34-10d95e923685",
    coverage = Coverage::Implemented,
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tourach's Chant",
        mana_cost = mana!("{1}{B}{B}"),
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
                cost: mana!("{B}"),
                effect: &Effect::SacrificeSelf,
            }],
        ),
        triggered!(
            Trigger::EntersBattlefield(&FOREST),
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
