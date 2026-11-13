//! Rust — {G} — Instant
//! Oracle: Counter target activated ability from an artifact source. (Mana abilities can't be targeted.)
//! Set: LEG #203 — Legends | Scryfall ID: ad4974c8-34c5-4290-b325-7586a67f6d56 | Oracle ID: 597ca66c-6bb1-4661-8ed9-8236f546e536

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUST,
    oracle_id = "597ca66c-6bb1-4661-8ed9-8236f546e536",
    scryfall_id = "ad4974c8-34c5-4290-b325-7586a67f6d56",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Rust",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::CounterTargetAbility],
        targets = Some(TargetReq::one(TargetSpec::AbilityOnStack(
            &Filter::ActivatedAbilityFrom(&Filter::ARTIFACT)
        )))
    ),],
);
