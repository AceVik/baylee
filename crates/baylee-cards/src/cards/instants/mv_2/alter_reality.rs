//! Alter Reality — {1}{U} — Instant
//! Oracle: Change the text of target spell or permanent by replacing all instances of one color word with another. (This effect lasts indefinitely.)
//! Oracle: Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: TOR #22 — Torment | Scryfall ID: 64cd68be-6e6a-4577-8465-a892463b6d6c | Oracle ID: 054c00b4-c940-455a-ad88-b8aa35931ecf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALTER_REALITY,
    oracle_id = "054c00b4-c940-455a-ad88-b8aa35931ecf",
    scryfall_id = "64cd68be-6e6a-4577-8465-a892463b6d6c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Alter Reality",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
