//! Ritual of the Machine — {2}{B}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Gain control of target nonartifact, nonblack creature.
//! Set: ME2 #109 — Masters Edition II | Scryfall ID: 5a691f5b-df18-4fb7-bad5-fe79a0072613 | Oracle ID: bccd8298-e056-4b91-b090-408c75467b92
// IMPLEMENTED — additional cost sacrificing a creature, and gaining control of target nonartifact, nonblack creature indefinitely.

use baylee_cards_dsl::prelude::*;

static NONARTIFACT_NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::ARTIFACT),
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

card!(
    index = index::RITUAL_OF_THE_MACHINE,
    oracle_id = "bccd8298-e056-4b91-b090-408c75467b92",
    scryfall_id = "5a691f5b-df18-4fb7-bad5-fe79a0072613",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ritual of the Machine",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)],
    ),],
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::GainControl,
            duration: Duration::Indefinitely,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &NONARTIFACT_NONBLACK_CREATURE,
        ))),
    )],
);
