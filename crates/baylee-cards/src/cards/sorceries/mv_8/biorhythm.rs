//! Biorhythm — {6}{G}{G} — Sorcery
//! Oracle: Each player's life total becomes the number of creatures they control.
//! Set: 9ED #231 — Ninth Edition | Scryfall ID: 17d1a10f-ce21-4914-9984-c7c559161230 | Oracle ID: 447de961-106c-4189-83ce-ca63d487e1cd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BIORHYTHM,
    oracle_id = "447de961-106c-4189-83ce-ca63d487e1cd",
    scryfall_id = "17d1a10f-ce21-4914-9984-c7c559161230",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Biorhythm",
        mana_cost = mana!("{6}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
