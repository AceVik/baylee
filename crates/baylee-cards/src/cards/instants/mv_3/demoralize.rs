//! Demoralize — {2}{R} — Instant
//! Oracle: All creatures gain menace until end of turn. (They can't be blocked except by two or more creatures.)
//! Oracle: Threshold — If there are seven or more cards in your graveyard, creatures can't block this turn.
//! Set: ODY #184 — Odyssey | Scryfall ID: 2e2f16f9-4980-45e9-99bf-0e00e3008b1d | Oracle ID: d0741810-0772-47f3-a709-4739fbf85576
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEMORALIZE,
    oracle_id = "d0741810-0772-47f3-a709-4739fbf85576",
    scryfall_id = "2e2f16f9-4980-45e9-99bf-0e00e3008b1d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Demoralize",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
