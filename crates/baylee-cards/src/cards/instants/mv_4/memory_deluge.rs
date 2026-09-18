//! Memory Deluge — {2}{U}{U} — Instant
//! Oracle: Look at the top X cards of your library, where X is the amount of mana spent to cast this spell. Put two of them into your hand and the rest on the bottom of your library in a random order.
//! Oracle: Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: INR #75 — Innistrad Remastered | Scryfall ID: edcd3802-ddb3-4eb6-9b6e-a26d76557662 | Oracle ID: e6fd55f2-7e26-469c-a44a-ea2eb90e19a9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MEMORY_DELUGE,
    oracle_id = "e6fd55f2-7e26-469c-a44a-ea2eb90e19a9",
    scryfall_id = "edcd3802-ddb3-4eb6-9b6e-a26d76557662",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Memory Deluge",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
