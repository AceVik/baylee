//! Wild Might — {1}{G} — Instant
//! Oracle: Target creature gets +1/+1 until end of turn. That creature gets an additional +4/+4 until end of turn unless any player pays {2}.
//! Set: PCY #134 — Prophecy | Scryfall ID: 7a8c0be2-ba90-4e7a-b1a6-c68be550e33d | Oracle ID: 3f8fa982-73ee-410a-8873-0afc8a6f11fc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WILD_MIGHT,
    oracle_id = "3f8fa982-73ee-410a-8873-0afc8a6f11fc",
    scryfall_id = "7a8c0be2-ba90-4e7a-b1a6-c68be550e33d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wild Might",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
