//! Aether Tide — {X}{U} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X creature cards.
//! Oracle: Return X target creatures to their owners' hands.
//! Set: EXO #27 — Exodus | Scryfall ID: 9aab7526-5825-4f31-92ff-be25ab5af2f5 | Oracle ID: 2fbf95b4-bcf4-4b5e-b5dc-0294f2b48d3e

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AETHER_TIDE,
    oracle_id = "2fbf95b4-bcf4-4b5e-b5dc-0294f2b48d3e",
    scryfall_id = "9aab7526-5825-4f31-92ff-be25ab5af2f5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Discarding X creature cards as an additional cost is not supported; returns X target creatures to their owners' hands without requiring the discard cost."
    ),
    faces = &[face!(
        name = "Aether Tide",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::bounce(TargetSpec::Object(&Filter::CREATURE))],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
