//! Call of the Herd — {2}{G} — Sorcery
//! Oracle: Create a 3/3 green Elephant creature token.
//! Oracle: Flashback {3}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: PLST #DMR-153 — The List | Scryfall ID: 892cd7b7-58d0-4bc4-a904-4b64fadd7f3c | Oracle ID: ee243f81-f51c-4d9a-a396-f7cef84b46c1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CALL_OF_THE_HERD,
    oracle_id = "ee243f81-f51c-4d9a-a396-f7cef84b46c1",
    scryfall_id = "892cd7b7-58d0-4bc4-a904-4b64fadd7f3c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Call of the Herd",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
