//! Psychic Purge — {U} — Sorcery
//! Oracle: Psychic Purge deals 1 damage to any target.
//! Oracle: When a spell or ability an opponent controls causes you to discard this card, that player loses 5 life.
//! Set: ME1 #45 — Masters Edition | Scryfall ID: 1f6e73a4-ad95-4c76-9aa5-94a439977204 | Oracle ID: 6af596d2-f075-4f55-b088-a5237fcdaa51
// PARTIAL — the 1 damage is written; the discard trigger is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PSYCHIC_PURGE,
    oracle_id = "6af596d2-f075-4f55-b088-a5237fcdaa51",
    scryfall_id = "1f6e73a4-ad95-4c76-9aa5-94a439977204",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "no Trigger names a discard and no TriggerZone is the hand, so the \
         ability that works while this card is being discarded cannot be \
         written"
    ),
    faces = &[face!(
        name = "Psychic Purge",
        mana_cost = mana!("{U}"),
        types = TypeSet::SORCERY,
    ),],
    // NOT SUPPORTED: "When a spell or ability an opponent controls causes you
    // to discard this card, that player loses 5 life." — `Trigger` has no
    // discard event (`CycledThis` reads a cycling discard alone), it has no
    // cause filter for "a spell or ability an opponent controls", and
    // `TriggerZone` is battlefield or graveyard, where this ability works
    // from the hand.
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::AnyTarget
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )],
);
