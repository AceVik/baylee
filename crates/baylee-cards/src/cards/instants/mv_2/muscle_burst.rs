//! Muscle Burst — {1}{G} — Instant
//! Oracle: Target creature gets +X/+X until end of turn, where X is 3 plus the number of cards named Muscle Burst in all graveyards.
//! Set: ODY #252 — Odyssey | Scryfall ID: 217dada5-7ffc-488b-8062-34c034906ea9 | Oracle ID: 97487ea5-2bbd-4ef6-a870-7e9f2db5e5e0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MUSCLE_BURST,
    oracle_id = "97487ea5-2bbd-4ef6-a870-7e9f2db5e5e0",
    scryfall_id = "217dada5-7ffc-488b-8062-34c034906ea9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Muscle Burst",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
