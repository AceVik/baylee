//! Entomb — {B} — Instant
//! Oracle: Search your library for a card, put that card into your graveyard, then shuffle.
//! Set: DMR #82 — Dominaria Remastered | Scryfall ID: 3caa9c55-5e3b-436b-84a9-b7ccebf63799 | Oracle ID: 299fc083-0834-4064-8344-f895aff68867
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENTOMB,
    oracle_id = "299fc083-0834-4064-8344-f895aff68867",
    scryfall_id = "3caa9c55-5e3b-436b-84a9-b7ccebf63799",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Entomb",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
