//! Morality Shift — {5}{B}{B} — Sorcery
//! Oracle: Exchange your graveyard and library. Then shuffle your library.
//! Set: JUD #70 — Judgment | Scryfall ID: b5c83e4d-ccc1-4ebf-9e74-2cc1ff9a7b07 | Oracle ID: 82e36ba9-9652-4594-8660-86c766b47709
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MORALITY_SHIFT,
    oracle_id = "82e36ba9-9652-4594-8660-86c766b47709",
    scryfall_id = "b5c83e4d-ccc1-4ebf-9e74-2cc1ff9a7b07",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Morality Shift",
        mana_cost = mana!("{5}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
