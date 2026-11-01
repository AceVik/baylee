//! Naked Singularity — {5} — Artifact
//! Oracle: Cumulative upkeep {3} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: If tapped for mana, Plains produce {R}, Islands produce {G}, Swamps produce {W}, Mountains produce {U}, and Forests produce {B} instead of any other type.
//! Set: ME4 #216 — Masters Edition IV | Scryfall ID: 5edddcbb-0e01-4473-b7a8-1e59cf7ee506 | Oracle ID: fb5ec55d-4a35-432a-be6e-c295f1b2e603
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NAKED_SINGULARITY,
    oracle_id = "fb5ec55d-4a35-432a-be6e-c295f1b2e603",
    scryfall_id = "5edddcbb-0e01-4473-b7a8-1e59cf7ee506",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    faces = &[face!(
        name = "Naked Singularity",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
