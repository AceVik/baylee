//! Complicate — {2}{U} — Instant
//! Oracle: Counter target spell unless its controller pays {3}.
//! Oracle: Cycling {2}{U} ({2}{U}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may counter target spell unless its controller pays {1}.
//! Set: ONS #76 — Onslaught | Scryfall ID: 33f69670-e494-42b8-9148-fe105ec61aa0 | Oracle ID: fc09cbc0-2d28-44c6-b9a9-b95e9e58bb1b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COMPLICATE,
    oracle_id = "fc09cbc0-2d28-44c6-b9a9-b95e9e58bb1b",
    scryfall_id = "33f69670-e494-42b8-9148-fe105ec61aa0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Complicate",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
