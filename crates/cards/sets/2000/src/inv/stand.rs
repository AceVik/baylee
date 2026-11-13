//! Stand // Deliver — {W} — Instant // Instant
//! Oracle: Prevent the next 2 damage that would be dealt to target creature this turn.
//! Oracle: Return target permanent to its owner's hand.
//! Set: DMR #207 — Dominaria Remastered | Scryfall ID: 6c9c00a1-8c24-4f35-aac9-00304687ee2b | Oracle ID: 6343aa78-2df6-41f3-9366-4933f9e6a93e
//! Face: Stand — {W} — Instant
//! Face: Deliver — {2}{U} — Instant
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STAND,
    oracle_id = "6343aa78-2df6-41f3-9366-4933f9e6a93e",
    scryfall_id = "6c9c00a1-8c24-4f35-aac9-00304687ee2b",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[
        face!(
            name = "Stand",
            mana_cost = mana!("{W}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Deliver",
            mana_cost = mana!("{2}{U}"),
            types = TypeSet::INSTANT,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
