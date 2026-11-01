//! Misfortune — {1}{B}{R}{G} — Sorcery
//! Oracle: An opponent chooses one —
//! Oracle: • You put a +1/+1 counter on each creature you control and gain 4 life.
//! Oracle: • You put a -1/-1 counter on each creature that player controls and Misfortune deals 4 damage to that player.
//! Set: ALL #113 — Alliances | Scryfall ID: b14cc32a-eb4f-4690-aceb-160780743ebe | Oracle ID: e282eb5b-f9e6-4fb3-9edf-44e23359357d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MISFORTUNE,
    oracle_id = "e282eb5b-f9e6-4fb3-9edf-44e23359357d",
    scryfall_id = "b14cc32a-eb4f-4690-aceb-160780743ebe",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    faces = &[face!(
        name = "Misfortune",
        mana_cost = mana!("{1}{B}{R}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
