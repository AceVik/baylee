//! Oubliette — {1}{B}{B} — Enchantment
//! Oracle: When this enchantment enters, target creature phases out until this enchantment leaves the battlefield. Tap that creature as it phases in this way. (Auras and Equipment phase out with it. While permanents are phased out, they're treated as though they don't exist.)
//! Set: 2XM #100 — Double Masters | Scryfall ID: d4800a7d-c229-4ced-97ff-0e58645d58d6 | Oracle ID: c753e9e3-9374-4e3c-8622-94576a8c1da3

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OUBLIETTE,
    oracle_id = "c753e9e3-9374-4e3c-8622-94576a8c1da3",
    scryfall_id = "d4800a7d-c229-4ced-97ff-0e58645d58d6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Oubliette",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::PhaseOutUntilSourceLeaves {
            tap_on_phase_in: true,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
