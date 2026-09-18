//! Consult the Star Charts — {1}{U} — Instant
//! Oracle: Kicker {1}{U} (You may pay an additional {1}{U} as you cast this spell.)
//! Oracle: Look at the top X cards of your library, where X is the number of lands you control. Put one of those cards into your hand. If this spell was kicked, put two of those cards into your hand instead. Put the rest on the bottom of your library in a random order.
//! Set: EOE #51 — Edge of Eternities | Scryfall ID: a16a6555-2e3a-4587-aacd-0307d696b26c | Oracle ID: e921839f-9d91-41a9-bc89-016af3c757aa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONSULT_THE_STAR_CHARTS,
    oracle_id = "e921839f-9d91-41a9-bc89-016af3c757aa",
    scryfall_id = "a16a6555-2e3a-4587-aacd-0307d696b26c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Consult the Star Charts",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
