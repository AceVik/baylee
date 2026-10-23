//! Subversion — {3}{B}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, each opponent loses 1 life. You gain life equal to the life lost this way.
//! Set: ULG #68 — Urza's Legacy | Scryfall ID: 50f1bca9-5831-4e8b-8920-f28ebb3ffb27 | Oracle ID: 0772c5d2-9189-47e2-b0ee-13babd94ea93
// PARTIAL — each opponent loses 1 life at your upkeep; gaining the life actually lost this way is unsupported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUBVERSION,
    oracle_id = "0772c5d2-9189-47e2-b0ee-13babd94ea93",
    scryfall_id = "50f1bca9-5831-4e8b-8920-f28ebb3ffb27",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "each opponent loses 1 life at your upkeep; gaining the life actually lost this way is unsupported"
    ),
    faces = &[face!(
        name = "Subversion",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::LoseLife {
            amount: Amount::Fixed(1),
            target: PlayerRel::EachOpponent
        }],
    )],
);
