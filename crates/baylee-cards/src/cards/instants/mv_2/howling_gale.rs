//! Howling Gale — {1}{G} — Instant
//! Oracle: Howling Gale deals 1 damage to each creature with flying and each player.
//! Oracle: Flashback {1}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #244 — Odyssey | Scryfall ID: 9917cf32-0236-4463-9b1d-e8193754ff97 | Oracle ID: 06118cec-ae63-4720-988d-df9cc15ccfab
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HOWLING_GALE,
    oracle_id = "06118cec-ae63-4720-988d-df9cc15ccfab",
    scryfall_id = "9917cf32-0236-4463-9b1d-e8193754ff97",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Howling Gale",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
