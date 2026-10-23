//! Mogg Maniac — {1}{R} — Creature — Goblin
//! Oracle: Whenever this creature is dealt damage, it deals that much damage to target opponent or planeswalker.
//! Set: TPR #147 — Tempest Remastered | Scryfall ID: df726d89-5246-4ece-89ff-f059e06e4460 | Oracle ID: 53b86704-4f16-498e-8c2c-e661cb85eb9c
// PARTIAL: whenever this creature is dealt damage, deals 1 damage to target opponent or planeswalker; reading the exact amount of damage dealt to a permanent in Trigger::DealtDamage is not supported

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOGG_MANIAC,
    oracle_id = "53b86704-4f16-498e-8c2c-e661cb85eb9c",
    scryfall_id = "df726d89-5246-4ece-89ff-f059e06e4460",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "whenever this creature is dealt damage, deals 1 damage to target opponent or planeswalker; reading the exact amount of damage dealt to a permanent in Trigger::DealtDamage is not supported"
    ),
    faces = &[face!(
        name = "Mogg Maniac",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::DealtDamage(&Filter::This),
        &[Effect::DealDamage {
            target: TargetSpec::OpponentOrObject(&Filter::PLANESWALKER),
            amount: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::one(TargetSpec::OpponentOrObject(
            &Filter::PLANESWALKER
        ))),
    )],
);
