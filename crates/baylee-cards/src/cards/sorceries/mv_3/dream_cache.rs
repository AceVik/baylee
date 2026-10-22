//! Dream Cache — {2}{U} — Sorcery
//! Oracle: Draw three cards, then put two cards from your hand both on top of your library or both on the bottom of your library.
//! Set: C18 #88 — Commander 2018 | Scryfall ID: d7da182c-cde3-40d8-976e-a88427a1edb8 | Oracle ID: 81338d64-8fc3-41ac-afc8-e2a79df1159b
// PARTIAL — puts two cards from your hand on top of your library; the option to put both on the bottom of your library instead is not in Effect.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DREAM_CACHE,
    oracle_id = "81338d64-8fc3-41ac-afc8-e2a79df1159b",
    scryfall_id = "d7da182c-cde3-40d8-976e-a88427a1edb8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "draws three cards and puts two cards from your hand on top of your library; the option to put both cards on the bottom of your library instead is not in Effect",
    ),
    faces = &[face!(
        name = "Dream Cache",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::draw(3),
        Effect::PutFromHandOnTop { count: 2 },
    ])],
);
