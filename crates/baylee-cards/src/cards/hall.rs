//! Hall — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R} or {W}.
//! Oracle: {4}, {T}: Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
//! Set: CLU #16 — Ravnica: Clue Edition | Scryfall ID: ab3d0e50-630a-4ccc-aa79-1db912ea801e | Oracle ID: 949e455d-6a9e-491a-892f-826cc8be0fd9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 595,
    oracle_id: "949e455d-6a9e-491a-892f-826cc8be0fd9",
    scryfall_id: "ab3d0e50-630a-4ccc-aa79-1db912ea801e",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::White]),
    faces: &[
    face! {
        name: "Hall",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
