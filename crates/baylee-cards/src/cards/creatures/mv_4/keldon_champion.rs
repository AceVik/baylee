//! Keldon Champion — {2}{R}{R} — Creature — Human Barbarian
//! Oracle: Haste
//! Oracle: Echo {2}{R}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, it deals 3 damage to target player or planeswalker.
//! Set: EMA #135 — Eternal Masters | Scryfall ID: 2ca0987e-7dff-410a-8328-45ba1c53e974 | Oracle ID: 1600b361-9298-49f0-8925-4c1aad363e1b
// IMPLEMENTED — haste, echo {2}{R}{R}, and ETB 3 damage to target player or planeswalker.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KELDON_CHAMPION,
    oracle_id = "1600b361-9298-49f0-8925-4c1aad363e1b",
    scryfall_id = "2ca0987e-7dff-410a-8328-45ba1c53e974",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Keldon Champion",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::BARBARIAN],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{2}{R}{R}"),
        },
        triggered!(
            Trigger::ETB,
            &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
            }],
            targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
                &Filter::PLANESWALKER
            ))),
        ),
    ],
);
