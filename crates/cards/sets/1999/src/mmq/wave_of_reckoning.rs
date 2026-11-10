//! Wave of Reckoning — {4}{W} — Sorcery
//! Oracle: Each creature deals damage to itself equal to its power.
//! Set: SOC #112 — Secrets of Strixhaven Commander | Scryfall ID: 7be0c25a-5e65-48e1-9560-9391111bb912 | Oracle ID: 24f7143c-d43a-4120-baa0-0235b15d8186
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAVE_OF_RECKONING,
    oracle_id = "24f7143c-d43a-4120-baa0-0235b15d8186",
    scryfall_id = "7be0c25a-5e65-48e1-9560-9391111bb912",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Wave of Reckoning",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
