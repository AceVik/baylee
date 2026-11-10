//! Time Elemental — {2}{U} — Creature — Elemental
//! Oracle: When this creature attacks or blocks, at end of combat, sacrifice it and it deals 5 damage to you.
//! Oracle: {2}{U}{U}, {T}: Return target permanent that isn't enchanted to its owner's hand.
//! Set: ME1 #53 — Masters Edition | Scryfall ID: 720672dc-d75a-44c3-a48d-d2fe3993fbbf | Oracle ID: d89075a3-4413-4796-a055-eef51fddb7f3

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TIME_ELEMENTAL,
    oracle_id = "d89075a3-4413-4796-a055-eef51fddb7f3",
    scryfall_id = "720672dc-d75a-44c3-a48d-d2fe3993fbbf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Time Elemental",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(0),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[
                    Effect::SacrificeSelf,
                    Effect::DealDamage {
                        amount: Amount::Fixed(5),
                        target: TargetSpec::Player(PlayerRel::You),
                    },
                ],
            }],
        ),
        triggered!(
            Trigger::Blocks(&Filter::This),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[
                    Effect::SacrificeSelf,
                    Effect::DealDamage {
                        amount: Amount::Fixed(5),
                        target: TargetSpec::Player(PlayerRel::You),
                    },
                ],
            }],
        ),
        activated!(
            cost!("{2}{U}{U}", TapSelf),
            &[Effect::bounce(TargetSpec::Object(&Filter::Not(
                &Filter::Enchanted
            )))],
            target = Some(TargetSpec::Object(&Filter::Not(&Filter::Enchanted)))
        ),
    ],
);
