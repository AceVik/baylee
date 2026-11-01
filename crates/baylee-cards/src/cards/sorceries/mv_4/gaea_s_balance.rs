//! Gaea's Balance — {3}{G} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice five lands.
//! Oracle: Search your library for a land card of each basic land type, put those cards onto the battlefield, then shuffle.
//! Set: APC #77 — Apocalypse | Scryfall ID: f1ffc5f8-ff1c-4733-b046-8679fa16371b | Oracle ID: 5aadbd03-c459-4893-aa1b-efcf193faa1b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GAEA_S_BALANCE,
    oracle_id = "5aadbd03-c459-4893-aa1b-efcf193faa1b",
    scryfall_id = "f1ffc5f8-ff1c-4733-b046-8679fa16371b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gaea's Balance",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
