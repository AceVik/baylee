//! Soul Strings — {X}{B} — Sorcery
//! Oracle: Return two target creature cards from your graveyard to your hand unless any player pays {X}.
//! Set: PCY #78 — Prophecy | Scryfall ID: e34f9d1b-a89a-439f-8aa9-b96a1bf892eb | Oracle ID: fac310c7-c1e5-42f9-8140-17472be21d0d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOUL_STRINGS,
    oracle_id = "fac310c7-c1e5-42f9-8140-17472be21d0d",
    scryfall_id = "e34f9d1b-a89a-439f-8aa9-b96a1bf892eb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Soul Strings",
        mana_cost = mana!("{X}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
