//! Order // Chaos — {3}{W} — Instant // Instant
//! Oracle: Exile target attacking creature.
//! Oracle: Creatures can't block this turn.
//! Set: DMR #212 — Dominaria Remastered | Scryfall ID: 8e4f2486-1a6f-414f-9122-134390fe3b0d | Oracle ID: bc540259-459d-4561-9268-c4f323ea62d2
//! Face: Order — {3}{W} — Instant
//! Face: Chaos — {2}{R} — Instant
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ORDER,
    oracle_id = "bc540259-459d-4561-9268-c4f323ea62d2",
    scryfall_id = "8e4f2486-1a6f-414f-9122-134390fe3b0d",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[
        face!(
            name = "Order",
            mana_cost = mana!("{3}{W}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Chaos",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::INSTANT,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
