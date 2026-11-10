//! Imperial Seal — {B} — Sorcery
//! Oracle: Search your library for a card, then shuffle and put that card on top. You lose 2 life.
//! Set: 2X2 #79 — Double Masters 2022 | Scryfall ID: e71a6bd3-7478-4c3a-8ae0-98352ce28492 | Oracle ID: 16cd0b90-f70c-4efa-b252-8de8784ef9a3
// IMPLEMENTED — search any card to the top of the library (the shuffle is
// derived), then lose 2 life.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IMPERIAL_SEAL,
    oracle_id = "16cd0b90-f70c-4efa-b252-8de8784ef9a3",
    scryfall_id = "e71a6bd3-7478-4c3a-8ae0-98352ce28492",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Imperial Seal",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::SearchLibrary {
            filter: &Filter::Any,
            finds: &[Find::TOP_OF_LIBRARY],
            optional: false,
        },
        Effect::LoseLife {
            amount: Amount::Fixed(2),
            target: PlayerRel::You,
        },
    ])],
);
