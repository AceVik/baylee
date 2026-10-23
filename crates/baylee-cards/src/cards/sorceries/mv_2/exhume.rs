//! Exhume — {1}{B} — Sorcery
//! Oracle: Each player puts a creature card from their graveyard onto the battlefield.
//! Set: USG #134 — Urza's Saga | Scryfall ID: a88b23ce-ce19-47da-b9f2-055a4d6bdc79 | Oracle ID: fbe61f74-1b3c-4e12-8758-7029872c9ff1
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EXHUME,
    oracle_id = "fbe61f74-1b3c-4e12-8758-7029872c9ff1",
    scryfall_id = "a88b23ce-ce19-47da-b9f2-055a4d6bdc79",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "each player choosing a single creature card from their graveyard to return to the battlefield is not in Effect; returns all creature cards from each player's graveyard to the battlefield",
    ),
    faces = &[face!(
        name = "Exhume",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::EachPlayerGraveyardToBattlefield {
        filter: &Filter::CREATURE,
    }])],
);
