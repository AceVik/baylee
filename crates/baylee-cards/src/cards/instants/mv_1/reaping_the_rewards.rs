//! Reaping the Rewards — {W} — Instant
//! Oracle: Buyback—Sacrifice a land. (You may sacrifice a land in addition to any other costs as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: You gain 2 life.
//! Set: EXO #16 — Exodus | Scryfall ID: 379b0495-8795-4b21-9d0a-dc4e10098de2 | Oracle ID: b4177445-46d8-4427-a345-d786c820c3af
// PARTIAL — Gains 2 life; Buyback—Sacrifice a land is not supported.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Gains 2 life; Buyback—Sacrifice a land is not supported.
card!(
    index = index::REAPING_THE_REWARDS,
    oracle_id = "b4177445-46d8-4427-a345-d786c820c3af",
    scryfall_id = "379b0495-8795-4b21-9d0a-dc4e10098de2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial("Gains 2 life; Buyback—Sacrifice a land is not supported."),
    faces = &[face!(
        name = "Reaping the Rewards",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::gain_life(2)]),],
);
