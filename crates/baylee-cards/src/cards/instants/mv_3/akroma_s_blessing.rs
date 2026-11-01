//! Akroma's Blessing — {2}{W} — Instant
//! Oracle: Choose a color. Creatures you control gain protection from the chosen color until end of turn.
//! Oracle: Cycling {W} ({W}, Discard this card: Draw a card.)
//! Set: VMA #11 — Vintage Masters | Scryfall ID: be43fdd6-917e-4a49-a9b8-d1673b421dce | Oracle ID: a3fc6888-0a4b-4e1d-b709-042b8b6b1a04
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AKROMA_S_BLESSING,
    oracle_id = "a3fc6888-0a4b-4e1d-b709-042b8b6b1a04",
    scryfall_id = "be43fdd6-917e-4a49-a9b8-d1673b421dce",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Akroma's Blessing",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
