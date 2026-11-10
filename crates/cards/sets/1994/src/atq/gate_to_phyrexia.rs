//! Gate to Phyrexia — {B}{B} — Enchantment
//! Oracle: Sacrifice a creature: Destroy target artifact. Activate only during your upkeep and only once each turn.
//! Set: ME4 #82 — Masters Edition IV | Scryfall ID: 4eb41639-b96b-4789-b488-24da503ef1e2 | Oracle ID: bbb005de-bbba-458e-87c1-912a004e80da
// IMPLEMENTED — "Sacrifice a creature: Destroy target artifact", gated to the
// controller's upkeep and once each turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GATE_TO_PHYREXIA,
    oracle_id = "bbb005de-bbba-458e-87c1-912a004e80da",
    scryfall_id = "4eb41639-b96b-4789-b488-24da503ef1e2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gate to Phyrexia",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(Sacrifice(&Filter::CREATURE)),
        &[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT))],
        target = Some(TargetSpec::Object(&Filter::ARTIFACT)),
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Upkeep)
        ])),
        limit = ActivationLimit::PerTurn(1),
    )],
);
