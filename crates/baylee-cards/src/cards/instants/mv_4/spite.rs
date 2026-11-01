//! Spite // Malice — {3}{U} — Instant // Instant
//! Oracle: Counter target noncreature spell.
//! Oracle: Destroy target nonblack creature. It can't be regenerated.
//! Set: DMR #208 — Dominaria Remastered | Scryfall ID: 1a83658a-8820-4b71-838f-fe1c2ef822d0 | Oracle ID: b829de81-04cd-498e-9f31-074f23e2a488
//! Face: Spite — {3}{U} — Instant
//! Face: Malice — {3}{B} — Instant
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPITE,
    oracle_id = "b829de81-04cd-498e-9f31-074f23e2a488",
    scryfall_id = "1a83658a-8820-4b71-838f-fe1c2ef822d0",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[
        face!(
            name = "Spite",
            mana_cost = mana!("{3}{U}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Malice",
            mana_cost = mana!("{3}{B}"),
            types = TypeSet::INSTANT,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
