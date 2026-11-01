//! Beast Attack — {2}{G}{G}{G} — Instant
//! Oracle: Create a 4/4 green Beast creature token.
//! Oracle: Flashback {2}{G}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: DDS #43 — Duel Decks: Mind vs. Might | Scryfall ID: 011f805f-3ebb-4748-a5de-8a10b5f47dcd | Oracle ID: 3f5acd90-199c-4ed3-9c8a-eeac4670d876
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BEAST_ATTACK,
    oracle_id = "3f5acd90-199c-4ed3-9c8a-eeac4670d876",
    scryfall_id = "011f805f-3ebb-4748-a5de-8a10b5f47dcd",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Beast Attack",
        mana_cost = mana!("{2}{G}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
