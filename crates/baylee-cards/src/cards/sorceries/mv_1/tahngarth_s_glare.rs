//! Tahngarth's Glare — {R} — Sorcery
//! Oracle: Look at the top three cards of target opponent's library, then put them back in any order. That player looks at the top three cards of your library, then puts them back in any order.
//! Set: APC #70 — Apocalypse | Scryfall ID: 442a4331-99ce-405e-b261-19b7f3375ddf | Oracle ID: 390cbc61-6291-4e7d-b262-4de57c827702
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TAHNGARTH_S_GLARE,
    oracle_id = "390cbc61-6291-4e7d-b262-4de57c827702",
    scryfall_id = "442a4331-99ce-405e-b261-19b7f3375ddf",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Tahngarth's Glare",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
