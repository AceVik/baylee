//! Mages' Contest — {1}{R}{R} — Instant
//! Oracle: You and target spell's controller bid life. You start the bidding with a bid of 1. In turn order, each player may top the high bid. The bidding ends if the high bid stands. The high bidder loses life equal to the high bid. If you win the bidding, counter that spell.
//! Set: INV #154 — Invasion | Scryfall ID: c516861c-68d9-4d02-a343-689dba0526c6 | Oracle ID: b0a6bf69-7e21-4d5c-ac72-5946d84fc2ca
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MAGES_CONTEST,
    oracle_id = "b0a6bf69-7e21-4d5c-ac72-5946d84fc2ca",
    scryfall_id = "c516861c-68d9-4d02-a343-689dba0526c6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mages' Contest",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
