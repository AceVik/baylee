//! Slithery Stalker — {1}{B}{B} — Creature — Nightmare Horror
//! Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.)
//! Oracle: When this creature enters, exile target green or white creature an opponent controls.
//! Oracle: When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
//! Set: TOR #84 — Torment | Scryfall ID: 0c875bb6-55bb-41cc-8825-8011581ff001 | Oracle ID: 0c31edd0-a471-4994-8209-b0a7370fc27b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SLITHERY_STALKER,
    oracle_id = "0c31edd0-a471-4994-8209-b0a7370fc27b",
    scryfall_id = "0c875bb6-55bb-41cc-8825-8011581ff001",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Slithery Stalker",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::HORROR],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
