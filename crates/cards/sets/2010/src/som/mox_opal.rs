//! Mox Opal — {0} — Legendary Artifact
//! Oracle: Metalcraft — {T}: Add one mana of any color. Activate only if you control three or more artifacts.
//! Set: 2XM #275 — Double Masters | Scryfall ID: 56001a36-126b-4c08-af98-a6cc4d84210e | Oracle ID: de2440de-e948-4811-903c-0bbe376ff64d
// IMPLEMENTED — metalcraft: the mana ability activates only with 3+
// artifacts under your control (Condition::ControlCount).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOX_OPAL,
    oracle_id = "de2440de-e948-4811-903c-0bbe376ff64d",
    scryfall_id = "56001a36-126b-4c08-af98-a6cc4d84210e",
    faces = &[face!(
        name = "Mox Opal",
        // Printed `{0}`, which is not the same thing as no cost at all
        // (CR 202.1b) — and `face!`'s default is the blank one. See
        // `casting::has_a_printed_cost`.
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
    )],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(
        Cost::TAP,
        &[Effect::mana_choice(ALL_MANA_COLORS)],
        condition = Some(Condition::ControlCount(&Filter::ARTIFACT, 3))
    )],
);
