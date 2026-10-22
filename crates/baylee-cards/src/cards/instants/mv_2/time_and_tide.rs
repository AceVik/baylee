//! Time and Tide — {U}{U} — Instant
//! Oracle: Simultaneously, all phased-out creatures phase in and all creatures with phasing phase out.
//! Set: VIS #46 — Visions | Scryfall ID: 152b348a-0301-4d45-a2c1-d78802c445ba | Oracle ID: b6fee94a-30b8-415e-8567-ad03ccb1d658
// PARTIAL: phases out all phased-in creatures with phasing; simultaneously phasing in all phased-out creatures is not yet modeled in Effect.

use baylee_cards_dsl::prelude::*;

static CREATURE_WITH_PHASING: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::PHASING)]);

card!(
    index = index::TIME_AND_TIDE,
    oracle_id = "b6fee94a-30b8-415e-8567-ad03ccb1d658",
    scryfall_id = "152b348a-0301-4d45-a2c1-d78802c445ba",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "phases out all phased-in creatures with phasing; simultaneously phasing in all phased-out creatures is not yet modeled in Effect",
    ),
    faces = &[face!(
        name = "Time and Tide",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::PhaseOutFilter(&CREATURE_WITH_PHASING)])],
);
