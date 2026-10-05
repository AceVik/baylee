//! Reverse Polarity — {W}{W} — Instant
//! Oracle: You gain X life, where X is twice the damage dealt to you so far this turn by artifacts.
//! Set: SUM #36 — Summer Magic / Edgar | Scryfall ID: 1e89856a-2496-41b2-b90c-ed42b02980cc | Oracle ID: c7076e2c-81f1-44ad-a4dd-45a01802d364
// PARTIAL — the whole spell is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REVERSE_POLARITY,
    oracle_id = "c7076e2c-81f1-44ad-a4dd-45a01802d364",
    scryfall_id = "1e89856a-2496-41b2-b90c-ed42b02980cc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Reverse Polarity",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "Amount::DamageDealtToYouThisTurn counts every source's damage to you \
         and carries no source filter, so \"by artifacts\" cannot be said; \
         and no Amount doubles another counted value (Amount::DoubleX is \
         twice the announced X, which this spell never announces)"
    ),
    // NOT SUPPORTED: "You gain X life, where X is twice the damage dealt to
    // you so far this turn by artifacts." — the damage half exists as
    // `Amount::DamageDealtToYouThisTurn`, but the engine counts it in
    // `GameState::damage_player` without asking what dealt the damage, so
    // there is no artifact-only reading; and no `Amount` wrapper doubles a
    // counted value (`Amount::DoubleX` and `Effect::GainLifeDoubleX` double
    // the announced X and nothing else). So the spell comes off the card.
    abilities = &[],
);
