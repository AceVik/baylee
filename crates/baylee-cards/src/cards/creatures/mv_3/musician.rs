//! Musician — {2}{U} — Creature — Human Wizard
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: {T}: Put a music counter on target creature. If it doesn't have "At the beginning of your upkeep, destroy this creature unless you pay {1} for each music counter on it," it gains that ability.
//! Set: ME2 #56 — Masters Edition II | Scryfall ID: 66d265d0-9237-4774-bfbf-d8771a583398 | Oracle ID: 0d6e9468-adb2-4feb-87cf-9113db8717b1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MUSICIAN,
    oracle_id = "0d6e9468-adb2-4feb-87cf-9113db8717b1",
    scryfall_id = "66d265d0-9237-4774-bfbf-d8771a583398",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Musician",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
