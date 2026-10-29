//! Twilight's Call — {4}{B}{B} — Sorcery
//! Oracle: You may cast this spell as though it had flash if you pay {2} more to cast it. (You may cast it any time you could cast an instant.)
//! Oracle: Each player returns all creature cards from their graveyard to the battlefield.
//! Set: DDJ #76 — Duel Decks: Izzet vs. Golgari | Scryfall ID: a6e04dd2-75ad-4427-93cc-37226340c2fb | Oracle ID: ae0a1d9c-19cb-42ee-97c3-464e38e84615
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TWILIGHT_S_CALL,
    oracle_id = "ae0a1d9c-19cb-42ee-97c3-464e38e84615",
    scryfall_id = "a6e04dd2-75ad-4427-93cc-37226340c2fb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Twilight's Call",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
