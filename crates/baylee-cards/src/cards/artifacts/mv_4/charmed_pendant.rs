//! Charmed Pendant — {4} — Artifact
//! Oracle: {T}, Mill a card: For each colored mana symbol in the milled card's mana cost, add one mana of that color. (Activate only as an instant. For example, if the card's mana cost is {3}{U}{U}{B}, you add {U}{U}{B}.)
//! Set: ODY #298 — Odyssey | Scryfall ID: 701942dd-7777-436f-8076-194584be8285 | Oracle ID: 12469cb5-a383-47ab-84cd-414b9ced8a8c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHARMED_PENDANT,
    oracle_id = "12469cb5-a383-47ab-84cd-414b9ced8a8c",
    scryfall_id = "701942dd-7777-436f-8076-194584be8285",
    faces = &[face!(
        name = "Charmed Pendant",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
