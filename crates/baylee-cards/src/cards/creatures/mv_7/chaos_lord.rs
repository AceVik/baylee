//! Chaos Lord — {4}{R}{R}{R} — Creature — Human
//! Oracle: First strike
//! Oracle: At the beginning of your upkeep, target opponent gains control of this creature if the number of permanents is even.
//! Oracle: This creature can attack as though it had haste unless it entered this turn.
//! Set: ICE #178 — Ice Age | Scryfall ID: ee245922-b380-4b2e-a43f-ab1ba8078943 | Oracle ID: 16d0aea2-4ca4-4a9d-8674-2863e085f514
// PARTIAL: even-permanent-count upkeep control change and non-entry haste not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: At the beginning of your upkeep, target opponent gains control of this creature if the number of permanents is even.
// NOT SUPPORTED: This creature can attack as though it had haste unless it entered this turn.
card!(
    index = index::CHAOS_LORD,
    oracle_id = "16d0aea2-4ca4-4a9d-8674-2863e085f514",
    scryfall_id = "ee245922-b380-4b2e-a43f-ab1ba8078943",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Partial(
        "even-permanent-count upkeep control change and non-entry haste not supported"
    ),
    faces = &[face!(
        name = "Chaos Lord",
        mana_cost = mana!("{4}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(7),
        toughness = Some(7),
    ),],
);
