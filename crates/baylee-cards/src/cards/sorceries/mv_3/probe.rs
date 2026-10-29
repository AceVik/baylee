//! Probe — {2}{U} — Sorcery
//! Oracle: Kicker {1}{B} (You may pay an additional {1}{B} as you cast this spell.)
//! Oracle: Draw three cards, then discard two cards. If this spell was kicked, target player discards two cards.
//! Set: INV #66 — Invasion | Scryfall ID: a2a58d18-3d52-4178-86b2-7590d4164e76 | Oracle ID: f8d227b6-1627-4ea1-b815-887094497abf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PROBE,
    oracle_id = "f8d227b6-1627-4ea1-b815-887094497abf",
    scryfall_id = "a2a58d18-3d52-4178-86b2-7590d4164e76",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Probe",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
