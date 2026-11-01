//! Hallowed Healer — {2}{W} — Creature — Human Cleric
//! Oracle: {T}: Prevent the next 2 damage that would be dealt to any target this turn.
//! Oracle: Threshold — {T}: Prevent the next 4 damage that would be dealt to any target this turn. Activate only if there are seven or more cards in your graveyard.
//! Set: ODY #25 — Odyssey | Scryfall ID: 254d5bf0-f985-4919-a142-d4578ae9a38e | Oracle ID: b92f6adb-64c6-4692-af9d-af4241203722
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HALLOWED_HEALER,
    oracle_id = "b92f6adb-64c6-4692-af9d-af4241203722",
    scryfall_id = "254d5bf0-f985-4919-a142-d4578ae9a38e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Hallowed Healer",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
