//! Fog Patch — {1}{G} — Instant
//! Oracle: Cast this spell only during the declare blockers step.
//! Oracle: Attacking creatures become blocked. (This spell works on creatures that can't be blocked.)
//! Set: NEM #104 — Nemesis | Scryfall ID: 133f9e4f-2b1b-4a24-ad19-285a2c5845b5 | Oracle ID: eb8ff6c1-f9b4-4dae-8f46-0f1ad4c05355
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FOG_PATCH,
    oracle_id = "eb8ff6c1-f9b4-4dae-8f46-0f1ad4c05355",
    scryfall_id = "133f9e4f-2b1b-4a24-ad19-285a2c5845b5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Fog Patch",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
