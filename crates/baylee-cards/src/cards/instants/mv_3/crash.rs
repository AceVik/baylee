//! Crash — {2}{R} — Instant
//! Oracle: You may sacrifice a Mountain rather than pay this spell's mana cost.
//! Oracle: Destroy target artifact.
//! Set: MMQ #186 — Mercadian Masques | Scryfall ID: 7a26bde3-8392-4476-b347-f223d52554a6 | Oracle ID: 562af5da-1080-46d2-8a38-71f06b7e8457
// NOT SUPPORTED: Sacrificing a Mountain rather than paying this spell's mana cost is not yet supported by cast_wizard::paid_as_an_alternative_cost; in play, Crash is cast for {2}{R} and destroys target artifact.
// PARTIAL — Sacrificing a Mountain rather than paying this spell's mana cost is not yet supported by cast_wizard::paid_as_an_alternative_cost; in play, Crash is cast for {2}{R} and destroys target artifact.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRASH,
    oracle_id = "562af5da-1080-46d2-8a38-71f06b7e8457",
    scryfall_id = "7a26bde3-8392-4476-b347-f223d52554a6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Sacrificing a Mountain rather than paying this spell's mana cost is not yet supported by cast_wizard::paid_as_an_alternative_cost; in play, Crash is cast for {2}{R} and destroys target artifact.",
    ),
    faces = &[face!(
        name = "Crash",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
    ),],
);
