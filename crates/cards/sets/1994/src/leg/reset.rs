//! Reset — {U}{U} — Instant
//! Oracle: Cast this spell only during an opponent's turn after their upkeep step.
//! Oracle: Untap all lands you control.
//! Set: ME3 #48 — Masters Edition III | Scryfall ID: 860aa0fe-0337-458c-b864-5ef5733fbae6 | Oracle ID: 4faa5a70-ac0c-4a29-8bec-218a35a29fdf
// IMPLEMENTED — castable only on an opponent's turn once its upkeep step has
// ended, untapping every land you control.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RESET,
    oracle_id = "4faa5a70-ac0c-4a29-8bec-218a35a29fdf",
    scryfall_id = "860aa0fe-0337-458c-b864-5ef5733fbae6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reset",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::UntapAll {
            filter: &Filter::YOUR_LAND,
        }],
        // "After their upkeep step" is the turn having reached its draw step,
        // the one CR 500.1 puts immediately after the upkeep: during the
        // upkeep `BeforeStep(Draw)` still holds, so its negation begins
        // exactly when the upkeep ends. `OpponentsTurn` supplies the seat the
        // same sentence names.
        condition = Some(Condition::All(&[
            Condition::OpponentsTurn,
            Condition::Not(&Condition::BeforeStep(StepKind::Draw)),
        ]))
    )],
);
