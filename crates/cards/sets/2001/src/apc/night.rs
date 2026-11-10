//! Night // Day — {B} — Instant // Instant
//! Oracle: Target creature gets -1/-1 until end of turn.
//! Oracle: Creatures target player controls get +1/+1 until end of turn.
//! Set: DMR #214 — Dominaria Remastered | Scryfall ID: eccff9be-a18d-4fad-b23a-7f66ae9dbfed | Oracle ID: cb32ab3e-ef76-4afb-959f-f8cc2d18416e
//! Face: Night — {B} — Instant
//! Face: Day — {2}{W} — Instant
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NIGHT,
    oracle_id = "cb32ab3e-ef76-4afb-959f-f8cc2d18416e",
    scryfall_id = "eccff9be-a18d-4fad-b23a-7f66ae9dbfed",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[
        face!(
            name = "Night",
            mana_cost = mana!("{B}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Day",
            mana_cost = mana!("{2}{W}"),
            types = TypeSet::INSTANT,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
