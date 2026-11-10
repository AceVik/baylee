//! Spinal Embrace — {3}{U}{U}{B} — Instant
//! Oracle: Cast this spell only during combat.
//! Oracle: Untap target creature you don't control and gain control of it. It gains haste until end of turn. At the beginning of the next end step, sacrifice it. If you do, you gain life equal to its toughness.
//! Set: DMR #201 — Dominaria Remastered | Scryfall ID: cd12b2d6-3940-4d47-8c76-ae73b8a14b15 | Oracle ID: 4cf3fb65-9107-428a-8853-029ec97112b5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPINAL_EMBRACE,
    oracle_id = "4cf3fb65-9107-428a-8853-029ec97112b5",
    scryfall_id = "cd12b2d6-3940-4d47-8c76-ae73b8a14b15",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Spinal Embrace",
        mana_cost = mana!("{3}{U}{U}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
