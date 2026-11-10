//! Blind Fury — {2}{R}{R} — Instant
//! Oracle: All creatures lose trample until end of turn. If a creature would deal combat damage to a creature this turn, it deals double that damage to that creature instead.
//! Set: MIR #158 — Mirage | Scryfall ID: 0ff91a17-0a5f-456f-a431-8505ba29e679 | Oracle ID: 5dbfd316-a0a9-4caa-99b1-069931d2aaa4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLIND_FURY,
    oracle_id = "5dbfd316-a0a9-4caa-99b1-069931d2aaa4",
    scryfall_id = "0ff91a17-0a5f-456f-a431-8505ba29e679",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Blind Fury",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
