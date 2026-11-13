//! Phyrexian Tribute — {2}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice two creatures.
//! Oracle: Destroy target artifact.
//! Set: MIR #134 — Mirage | Scryfall ID: bfc55d48-6d6f-429d-8281-e66a9996d574 | Oracle ID: c06b993b-a36a-412a-a732-d678a4179791
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PHYREXIAN_TRIBUTE,
    oracle_id = "c06b993b-a36a-412a-a732-d678a4179791",
    scryfall_id = "bfc55d48-6d6f-429d-8281-e66a9996d574",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Phyrexian Tribute",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[
            CostPart::Sacrifice(&Filter::CREATURE),
            CostPart::Sacrifice(&Filter::CREATURE),
        ],
    ),],
    abilities = &[spell!(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::ARTIFACT),
            no_regen: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
    )],
);
