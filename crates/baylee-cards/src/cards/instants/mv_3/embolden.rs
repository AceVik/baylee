//! Embolden — {2}{W} — Instant
//! Oracle: Prevent the next 4 damage that would be dealt this turn to any number of targets, divided as you choose.
//! Oracle: Flashback {1}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #22 — Odyssey | Scryfall ID: 36848fb0-4070-40cf-b24a-2e8f47c5ebc3 | Oracle ID: 9904873c-7669-4485-b380-f28ff529b45c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EMBOLDEN,
    oracle_id = "9904873c-7669-4485-b380-f28ff529b45c",
    scryfall_id = "36848fb0-4070-40cf-b24a-2e8f47c5ebc3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Embolden",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
