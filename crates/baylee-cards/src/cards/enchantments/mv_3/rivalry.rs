//! Rivalry — {2}{R} — Enchantment
//! Oracle: At the beginning of each player's upkeep, if that player controls more lands than each other player, this enchantment deals 2 damage to them.
//! Set: ULG #90 — Urza's Legacy | Scryfall ID: 3b79677b-d5b9-47c7-a5f5-45446d5cddff | Oracle ID: da77cf1b-eaeb-4a60-aa16-a1217ce8b550
// NOT SUPPORTED: the intervening-if condition checking whether the active player controls more lands than each other player is not in Condition.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIVALRY,
    oracle_id = "da77cf1b-eaeb-4a60-aa16-a1217ce8b550",
    scryfall_id = "3b79677b-d5b9-47c7-a5f5-45446d5cddff",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "triggers at the beginning of each player's upkeep to deal 2 damage to that player; the intervening-if condition checking whether that player controls more lands than each other player is not in Condition",
    ),
    faces = &[face!(
        name = "Rivalry",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::ActivePlayer),
        }],
    )],
);
