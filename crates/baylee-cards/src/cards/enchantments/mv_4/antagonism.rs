//! Antagonism — {3}{R} — Enchantment
//! Oracle: At the beginning of each player's end step, this enchantment deals 2 damage to that player unless one of their opponents was dealt damage this turn.
//! Set: USG #173 — Urza's Saga | Scryfall ID: 6b9db511-089e-413f-8005-ccb05ec3b06e | Oracle ID: 68f81fcf-2dcf-41b9-ad7c-588149feb12a
// PARTIAL: Deals 2 damage to the active player at the beginning of each player's end step unconditionally; checking whether one of that player's opponents was dealt damage this turn is not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANTAGONISM,
    oracle_id = "68f81fcf-2dcf-41b9-ad7c-588149feb12a",
    scryfall_id = "6b9db511-089e-413f-8005-ccb05ec3b06e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Deals 2 damage to the active player at the beginning of each player's end step unconditionally; checking whether one of that player's opponents was dealt damage this turn is not supported.",
    ),
    faces = &[face!(
        name = "Antagonism",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::ActivePlayer),
        }],
    )],
);
