//! Timetwister — {2}{U} — Sorcery
//! Oracle: Each player shuffles their hand and graveyard into their library, then draws seven cards. (Then put Timetwister into its owner's graveyard.)
//! Set: VMA #3 — Vintage Masters | Scryfall ID: fbee1e10-0b8c-44ea-b0e5-44cdd0bfcd76 | Oracle ID: c823e687-6311-4c99-974b-fd77d204141a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIMETWISTER,
    oracle_id = "c823e687-6311-4c99-974b-fd77d204141a",
    scryfall_id = "fbee1e10-0b8c-44ea-b0e5-44cdd0bfcd76",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Timetwister",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
