//! Final Fortune — {R}{R} — Instant
//! Oracle: Take an extra turn after this one. At the beginning of that turn's end step, you lose the game.
//! Set: 7ED #182 — Seventh Edition | Scryfall ID: fb80afc3-4887-42a0-afb2-7fa997981fb2 | Oracle ID: 8d0adc5c-3fdd-4e22-b783-5651f8e57b65
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FINAL_FORTUNE,
    oracle_id = "8d0adc5c-3fdd-4e22-b783-5651f8e57b65",
    scryfall_id = "fb80afc3-4887-42a0-afb2-7fa997981fb2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Final Fortune",
        mana_cost = mana!("{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[
        Effect::TakeExtraTurn,
        // "That turn" is the next turn, because the extra turn comes right
        // after this one: at its upkeep, set the loss for its end step.
        Effect::AtNextTurnsUpkeep {
            effects: &[Effect::AtNextEndStep {
                effects: &[Effect::LoseGame],
            }],
        },
    ])],
);
