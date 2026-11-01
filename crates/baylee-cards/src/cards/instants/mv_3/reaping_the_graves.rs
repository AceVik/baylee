//! Reaping the Graves — {2}{B} — Instant
//! Oracle: Return target creature card from your graveyard to your hand.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: SCG #72 — Scourge | Scryfall ID: 760a66bd-2821-4710-8f02-3c30772dd884 | Oracle ID: 80c663a5-2794-4748-b47a-34352fb2112f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REAPING_THE_GRAVES,
    oracle_id = "80c663a5-2794-4748-b47a-34352fb2112f",
    scryfall_id = "760a66bd-2821-4710-8f02-3c30772dd884",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Reaping the Graves",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
