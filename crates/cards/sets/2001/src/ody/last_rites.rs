//! Last Rites — {2}{B} — Sorcery
//! Oracle: Discard any number of cards. Target player reveals their hand, then you choose a nonland card from it for each card discarded this way. That player discards those cards.
//! Set: ODY #146 — Odyssey | Scryfall ID: 0b140a36-6686-4781-a432-cb2ef58afa81 | Oracle ID: 3196e1f4-7f94-4eb6-ae2b-ed11ba392552
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAST_RITES,
    oracle_id = "3196e1f4-7f94-4eb6-ae2b-ed11ba392552",
    scryfall_id = "0b140a36-6686-4781-a432-cb2ef58afa81",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Last Rites",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
