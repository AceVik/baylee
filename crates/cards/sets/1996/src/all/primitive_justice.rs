//! Primitive Justice — {1}{R} — Sorcery
//! Oracle: As an additional cost to cast this spell, you may pay {1}{R} and/or {1}{G} any number of times.
//! Oracle: Destroy target artifact. For each additional {1}{R} you paid, destroy another target artifact. For each additional {1}{G} you paid, destroy another target artifact, and you gain 1 life.
//! Set: ME4 #130 — Masters Edition IV | Scryfall ID: 5bfe2504-5085-4dbd-b093-f0a1536b4526 | Oracle ID: cf38ac6e-e370-4ca5-90cd-26a1b812c63a
// PARTIAL — destroys one target artifact; the extra {1}{R} / {1}{G} payments (more artifacts, life) are not offered.
// NOT SUPPORTED: paying {1}{R} and/or {1}{G} any number of times as an additional cost to destroy an additional target artifact per payment and gain 1 life per {1}{G} paid has no DSL representation.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRIMITIVE_JUSTICE,
    oracle_id = "cf38ac6e-e370-4ca5-90cd-26a1b812c63a",
    scryfall_id = "5bfe2504-5085-4dbd-b093-f0a1536b4526",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Partial(
        "Paying {1}{R} and/or {1}{G} any number of times as an additional cost to destroy an additional target artifact per payment and gain 1 life per {1}{G} paid has no DSL representation; in play, Primitive Justice destroys one target artifact.",
    ),
    faces = &[face!(
        name = "Primitive Justice",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
    )],
);
