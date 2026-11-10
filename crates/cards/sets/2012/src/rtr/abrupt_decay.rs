//! Abrupt Decay — {B}{G} — Instant
//! Oracle: This spell can't be countered.
//! Oracle: Destroy target nonland permanent with mana value 3 or less.
//! Set: MM3 #146 — Modern Masters 2017 | Scryfall ID: a8e328c6-3a84-49cf-a1a3-1d1e5373d274 | Oracle ID: 1c747fe2-289e-492a-a846-aa77707e2dc3
// IMPLEMENTED — uncounterable single-target destroy, capped at mana value 3.

use baylee_cards_dsl::prelude::*;

/// "Nonland permanent with mana value 3 or less."
///
/// `TargetSpec::Object` already draws its options from the battlefield, so
/// "permanent" costs no clause; the two that are printed are the two here.
static SMALL_NONLAND: Filter = Filter::And(&[Filter::NONLAND, Filter::CmcAtMost(3)]);

card!(
    index = index::ABRUPT_DECAY,
    oracle_id = "1c747fe2-289e-492a-a846-aa77707e2dc3",
    scryfall_id = "a8e328c6-3a84-49cf-a1a3-1d1e5373d274",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Abrupt Decay",
        mana_cost = mana!("{B}{G}"),
        types = TypeSet::INSTANT,
    ),],
    keywords = KeywordSet::UNCOUNTERABLE,
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&SMALL_NONLAND))],
        targets = Some(TargetReq::one(TargetSpec::Object(&SMALL_NONLAND)))
    )],
);
