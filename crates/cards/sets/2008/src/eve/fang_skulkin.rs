//! Fang Skulkin — {2} — Artifact Creature — Scarecrow
//! Oracle: {2}: Target black creature gains wither until end of turn. (It deals damage to creatures in the form of -1/-1 counters.)
//! Set: EVE #168 — Eventide | Scryfall ID: ff5ed574-4ff2-4f67-8989-d3e47ba1d750 | Oracle ID: 54bed864-fe2e-4359-b0ac-c27a2c225487
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FANG_SKULKIN,
    oracle_id = "54bed864-fe2e-4359-b0ac-c27a2c225487",
    scryfall_id = "ff5ed574-4ff2-4f67-8989-d3e47ba1d750",
    faces = &[face!(
        name = "Fang Skulkin",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SCARECROW],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
