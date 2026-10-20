//! Arctic Foxes — {1}{W} — Creature — Fox
//! Oracle: This creature can't be blocked by creatures with power 2 or greater as long as defending player controls a snow land.
//! Set: ICE #2 — Ice Age | Scryfall ID: 98f99c3e-dddc-492f-aab6-1d899346a385 | Oracle ID: 898f0824-cf4c-4e34-8407-9a32fdae6dcb

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "as long as defending player controls a snow land" —
// Condition has no variant checking whether the defending player controls a matching permanent.
card!(
    index = index::ARCTIC_FOXES,
    oracle_id = "898f0824-cf4c-4e34-8407-9a32fdae6dcb",
    scryfall_id = "98f99c3e-dddc-492f-aab6-1d899346a385",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "missing condition checking whether defending player controls a snow land"
    ),
    faces = &[face!(
        name = "Arctic Foxes",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FOX],
        power = Some(1),
        toughness = Some(1),
    ),],
);
