//! Primal Boost — {2}{G} — Instant
//! Oracle: Target creature gets +4/+4 until end of turn.
//! Oracle: Cycling {2}{G} ({2}{G}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may have target creature get +1/+1 until end of turn.
//! Set: DMR #173 — Dominaria Remastered | Scryfall ID: 59f93eb0-cb0c-4ee0-8006-3d3c7515d254 | Oracle ID: 902e7d4b-7ab4-40b5-b3a5-c83ef604989a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRIMAL_BOOST,
    oracle_id = "902e7d4b-7ab4-40b5-b3a5-c83ef604989a",
    scryfall_id = "59f93eb0-cb0c-4ee0-8006-3d3c7515d254",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Primal Boost",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
