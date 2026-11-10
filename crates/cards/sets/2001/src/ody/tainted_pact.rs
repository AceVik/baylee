//! Tainted Pact — {1}{B} — Instant
//! Oracle: Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.
//! Set: ODY #164 — Odyssey | Scryfall ID: c513f51b-a0db-4c08-8acc-1e91060b93b7 | Oracle ID: 1a85ba2b-ae10-4917-954a-7709b75a9740
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TAINTED_PACT,
    oracle_id = "1a85ba2b-ae10-4917-954a-7709b75a9740",
    scryfall_id = "c513f51b-a0db-4c08-8acc-1e91060b93b7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tainted Pact",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
