//! Reverent Mantra — {3}{W} — Instant
//! Oracle: You may exile a white card from your hand rather than pay this spell's mana cost.
//! Oracle: Choose a color. All creatures gain protection from the chosen color until end of turn.
//! Set: MMQ #44 — Mercadian Masques | Scryfall ID: 48364e19-a3ea-4980-925f-7918e57315f1 | Oracle ID: b4f7b1eb-6cd4-40f9-b259-3c39291edf27
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REVERENT_MANTRA,
    oracle_id = "b4f7b1eb-6cd4-40f9-b259-3c39291edf27",
    scryfall_id = "48364e19-a3ea-4980-925f-7918e57315f1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Reverent Mantra",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
