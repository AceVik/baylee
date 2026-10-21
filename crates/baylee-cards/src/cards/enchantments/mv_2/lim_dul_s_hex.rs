//! Lim-Dûl's Hex — {1}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, for each player, this enchantment deals 1 damage to that player unless they pay {B} or {3}.
//! Set: ICE #146 — Ice Age | Scryfall ID: af976f42-3d56-4e32-8294-970a276a4bf3 | Oracle ID: 7f9116e1-9aab-470b-90eb-f51a3fbb3c0e
// NOT SUPPORTED: each player's option to pay {B} or {3} is not supported; deals 1 damage to each player at the beginning of your upkeep unconditionally.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIM_DUL_S_HEX,
    oracle_id = "7f9116e1-9aab-470b-90eb-f51a3fbb3c0e",
    scryfall_id = "af976f42-3d56-4e32-8294-970a276a4bf3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "each player's option to pay {B} or {3} is not supported; deals 1 damage to each player at the beginning of your upkeep unconditionally",
    ),
    faces = &[face!(
        name = "Lim-Dûl's Hex",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Player(PlayerRel::EachPlayer),
        }],
    )],
);
