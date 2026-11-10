//! Rites of Spring — {1}{G} — Sorcery
//! Oracle: Discard any number of cards. Search your library for up to that many basic land cards, reveal them, put them into your hand, then shuffle.
//! Set: ODY #265 — Odyssey | Scryfall ID: 1bddb0ef-fdda-491b-a0cf-48cbdd761918 | Oracle ID: a7f682ce-3d1f-425c-b65a-11a15791aa9e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RITES_OF_SPRING,
    oracle_id = "a7f682ce-3d1f-425c-b65a-11a15791aa9e",
    scryfall_id = "1bddb0ef-fdda-491b-a0cf-48cbdd761918",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Rites of Spring",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
