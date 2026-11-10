//! Psychic Purge — {U} — Sorcery
//! Oracle: Psychic Purge deals 1 damage to any target.
//! Oracle: When a spell or ability an opponent controls causes you to discard this card, that player loses 5 life.
//! Set: ME1 #45 — Masters Edition | Scryfall ID: 1f6e73a4-ad95-4c76-9aa5-94a439977204 | Oracle ID: 6af596d2-f075-4f55-b088-a5237fcdaa51
// IMPLEMENTED — damage and opponent-caused discard trigger.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PSYCHIC_PURGE,
    oracle_id = "6af596d2-f075-4f55-b088-a5237fcdaa51",
    scryfall_id = "1f6e73a4-ad95-4c76-9aa5-94a439977204",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Psychic Purge",
        mana_cost = mana!("{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[
        triggered!(
            Trigger::DiscardedThisByOpponent,
            &[Effect::LoseLife {
                amount: Amount::Fixed(5),
                target: PlayerRel::EventPlayer
            }]
        ),
        spell!(
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        )
    ],
);
