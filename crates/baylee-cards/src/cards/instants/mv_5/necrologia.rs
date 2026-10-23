//! Necrologia — {3}{B}{B} — Instant
//! Oracle: Cast this spell only during your end step.
//! Oracle: As an additional cost to cast this spell, pay X life.
//! Oracle: Draw X cards.
//! Set: TPR #110 — Tempest Remastered | Scryfall ID: d3e1f268-9861-40bf-85a7-2d33f0972e23 | Oracle ID: 925f11c3-fd32-4aa7-a130-6554b81ad3eb

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NECROLOGIA,
    oracle_id = "925f11c3-fd32-4aa7-a130-6554b81ad3eb",
    scryfall_id = "d3e1f268-9861-40bf-85a7-2d33f0972e23",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Restricting this spell to be cast only during your end step is not supported; it can be cast any time you could cast an instant to pay X life and draw X cards."
    ),
    faces = &[face!(
        name = "Necrologia",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::PayLifeX],
    ),],
    abilities = &[spell!(&[Effect::DrawCards { amount: Amount::X }])],
);
