//! Maelstrom Pulse — {1}{B}{G} — Sorcery
//! Oracle: Destroy target nonland permanent and all other permanents with the same name as that permanent.
//! Set: INR #244 — Innistrad Remastered | Scryfall ID: 66f17263-b916-40f4-b175-fcfd5630103d | Oracle ID: 95ce305f-34bc-4d6d-b7ba-ffd4b2a25336
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MAELSTROM_PULSE,
    oracle_id = "95ce305f-34bc-4d6d-b7ba-ffd4b2a25336",
    scryfall_id = "66f17263-b916-40f4-b175-fcfd5630103d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Maelstrom Pulse",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
