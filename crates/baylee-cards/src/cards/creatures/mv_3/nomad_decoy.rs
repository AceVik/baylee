//! Nomad Decoy — {2}{W} — Creature — Human Nomad
//! Oracle: {W}, {T}: Tap target creature.
//! Oracle: Threshold — {W}{W}, {T}: Tap two target creatures. Activate only if there are seven or more cards in your graveyard.
//! Set: DMR #17 — Dominaria Remastered | Scryfall ID: da039589-f3ee-4aff-b8b0-e5fda13ebe97 | Oracle ID: 73c40477-7061-47db-af8f-2a5cf6c22c9b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NOMAD_DECOY,
    oracle_id = "73c40477-7061-47db-af8f-2a5cf6c22c9b",
    scryfall_id = "da039589-f3ee-4aff-b8b0-e5fda13ebe97",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Nomad Decoy",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOMAD],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
