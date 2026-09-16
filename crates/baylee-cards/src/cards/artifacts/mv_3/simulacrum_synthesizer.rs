//! Simulacrum Synthesizer — {2}{U} — Artifact
//! Oracle: When this artifact enters, scry 2.
//! Oracle: Whenever another artifact you control with mana value 3 or greater enters, create a 0/0 colorless Construct artifact creature token with "This token gets +1/+1 for each artifact you control."
//! Set: BIG #6 — The Big Score | Scryfall ID: aaa05ad1-5cda-4edd-b6bf-562ae3e5011a | Oracle ID: eb7a1f21-a66d-415b-8520-710b44890bb6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SIMULACRUM_SYNTHESIZER,
    oracle_id = "eb7a1f21-a66d-415b-8520-710b44890bb6",
    scryfall_id = "aaa05ad1-5cda-4edd-b6bf-562ae3e5011a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Simulacrum Synthesizer",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
