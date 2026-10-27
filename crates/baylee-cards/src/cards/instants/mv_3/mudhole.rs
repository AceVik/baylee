//! Mudhole — {2}{R} — Instant
//! Oracle: Target player exiles all land cards from their graveyard.
//! Set: ODY #208 — Odyssey | Scryfall ID: 993d31bc-7355-4dfc-ac4e-ababaa0dc529 | Oracle ID: ce5ca56a-fce8-4e19-807f-7b8638fe4bcf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MUDHOLE,
    oracle_id = "ce5ca56a-fce8-4e19-807f-7b8638fe4bcf",
    scryfall_id = "993d31bc-7355-4dfc-ac4e-ababaa0dc529",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mudhole",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
