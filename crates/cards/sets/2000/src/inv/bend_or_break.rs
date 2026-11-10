//! Bend or Break — {3}{R} — Sorcery
//! Oracle: Each player separates all nontoken lands they control into two piles. For each player, one of their piles is chosen by one of their opponents of their choice. Destroy all lands in the chosen piles. Tap all lands in the other piles.
//! Set: INV #137 — Invasion | Scryfall ID: b76b6660-d4b2-44de-a1a7-8d00811f90f6 | Oracle ID: 4362efe4-339d-40e1-bd72-3cc9e41bf93f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BEND_OR_BREAK,
    oracle_id = "4362efe4-339d-40e1-bd72-3cc9e41bf93f",
    scryfall_id = "b76b6660-d4b2-44de-a1a7-8d00811f90f6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Bend or Break",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
