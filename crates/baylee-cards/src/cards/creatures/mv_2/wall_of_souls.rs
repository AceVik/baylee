//! Wall of Souls — {1}{B} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Whenever this creature is dealt combat damage, it deals that much damage to target opponent or planeswalker.
//! Set: TPR #123 — Tempest Remastered | Scryfall ID: a5b8d9a2-91d7-4172-b778-b5dc3ac28abe | Oracle ID: 2c85c9ea-c3fd-4981-a65c-4405bd87764e
// PARTIAL: restricting the trigger to combat damage and reading the exact amount of damage dealt to a permanent in Trigger::DealtDamage are not supported; has defender and deals 1 damage to target opponent or planeswalker whenever dealt damage.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_SOULS,
    oracle_id = "2c85c9ea-c3fd-4981-a65c-4405bd87764e",
    scryfall_id = "a5b8d9a2-91d7-4172-b778-b5dc3ac28abe",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "restricting the trigger to combat damage and reading the exact amount of damage dealt to a permanent in Trigger::DealtDamage are not supported; has defender and deals 1 damage to target opponent or planeswalker whenever dealt damage",
    ),
    keywords = KeywordSet::DEFENDER,
    faces = &[face!(
        name = "Wall of Souls",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(4),
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
    ),],
);
