//! Necrologia — {3}{B}{B} — Instant
//! Oracle: Cast this spell only during your end step.
//! Oracle: As an additional cost to cast this spell, pay X life.
//! Oracle: Draw X cards.
//! Set: TPR #110 — Tempest Remastered | Scryfall ID: d3e1f268-9861-40bf-85a7-2d33f0972e23 | Oracle ID: 925f11c3-fd32-4aa7-a130-6554b81ad3eb
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NECROLOGIA,
    oracle_id = "925f11c3-fd32-4aa7-a130-6554b81ad3eb",
    scryfall_id = "d3e1f268-9861-40bf-85a7-2d33f0972e23",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Necrologia",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::PayLifeX],
    ),],
    abilities = &[spell!(
        &[Effect::DrawCards { amount: Amount::X }],
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::End)
        ])),
    )],
);
