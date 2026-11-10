//! Guerrilla Tactics — {1}{R} — Instant
//! Oracle: Guerrilla Tactics deals 2 damage to any target.
//! Oracle: When a spell or ability an opponent controls causes you to discard this card, it deals 4 damage to any target.
//! Set: 10E #211 — Tenth Edition | Scryfall ID: 9bf3bac0-6e63-4bd3-bbd6-547f46c2d126 | Oracle ID: 65ad90f5-4dae-494b-83fb-8052e77e0b12
use baylee_cards_dsl::prelude::*;

card!(
    index = index::GUERRILLA_TACTICS,
    oracle_id = "65ad90f5-4dae-494b-83fb-8052e77e0b12",
    scryfall_id = "9bf3bac0-6e63-4bd3-bbd6-547f46c2d126",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Guerrilla Tactics",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
        triggered!(
            Trigger::DiscardedThisByOpponent,
            &[Effect::DealDamage {
                amount: Amount::Fixed(4),
                target: TargetSpec::AnyTarget,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
    ],
);
