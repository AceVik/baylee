//! Glasses of Urza — {1} — Artifact
//! Oracle: {T}: Look at target player's hand.
//! Set: ME4 #203 — Masters Edition IV | Scryfall ID: 4bd9f45f-30b3-4bff-9fd3-9a71137ac741 | Oracle ID: af7fabf4-8d55-4b06-9c21-472f4a5775b4
// PARTIAL — looking at another player's hand is not in the engine; it does
// nothing.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLASSES_OF_URZA,
    oracle_id = "af7fabf4-8d55-4b06-9c21-472f4a5775b4",
    scryfall_id = "4bd9f45f-30b3-4bff-9fd3-9a71137ac741",
    coverage =
        Coverage::Partial("looking at another player's hand is not in the engine; it does nothing"),
    faces = &[face!(
        name = "Glasses of Urza",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        // NOT SUPPORTED: {T}: Look at target player's hand.
    ],
);
