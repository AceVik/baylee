//! Cave-In — {3}{R}{R} — Sorcery
//! Oracle: You may exile a red card from your hand rather than pay this spell's mana cost.
//! Oracle: Cave-In deals 2 damage to each creature and each player.
//! Set: MMQ #180 — Mercadian Masques | Scryfall ID: 440d9d26-f304-467d-af79-914cc65f082e | Oracle ID: 7d937265-02f5-4f2c-9fef-c5930e13b3be
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static RED_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Red]));

card!(
    index = index::CAVE_IN,
    oracle_id = "7d937265-02f5-4f2c-9fef-c5930e13b3be",
    scryfall_id = "440d9d26-f304-467d-af79-914cc65f082e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cave-In",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::SORCERY,
        alternative_costs = &[AlternativeCost {
            cost: cost!(ExileFromHand(&RED_CARD)),
            condition: AltCondition::Always,
        }],
    ),],
    abilities = &[spell!(&[
        Effect::DealDamageEach {
            amount: Amount::Fixed(2),
            filter: &Filter::CREATURE,
        },
        Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::EachPlayer),
        },
    ])],
);
