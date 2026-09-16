//! Braided Net // Braided Quipu — {2}{U} — Artifact // Artifact
//! Oracle: This artifact enters with three net counters on it.
//! Oracle: {T}, Remove a net counter from this artifact: Tap another target nonland permanent. Its activated abilities can't be activated for as long as it remains tapped.
//! Oracle: Craft with artifact {1}{U}
//! Oracle: {3}{U}, {T}: Draw a card for each artifact you control, then put this artifact into its owner's library third from the top.
//! Set: LCI #47 — The Lost Caverns of Ixalan | Scryfall ID: 68a6ede0-6d57-4e29-9e3b-3569ab7f0bcd | Oracle ID: 86ef06b3-2049-494d-9ecb-ca14766d3b68
//! Face: Braided Net — {2}{U} — Artifact
//! Face: Braided Quipu —  — Artifact
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRAIDED_NET,
    oracle_id = "86ef06b3-2049-494d-9ecb-ca14766d3b68",
    scryfall_id = "68a6ede0-6d57-4e29-9e3b-3569ab7f0bcd",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Braided Net",
            mana_cost = mana!("{2}{U}"),
            types = TypeSet::ARTIFACT,
        ),
        face!(
            name = "Braided Quipu",
            types = TypeSet::ARTIFACT,
            castable_from_hand = false,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
