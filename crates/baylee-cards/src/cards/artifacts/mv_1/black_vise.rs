//! Black Vise — {1} — Artifact
//! Oracle: As this artifact enters, choose an opponent.
//! Oracle: At the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is the number of cards in their hand minus 4.
//! Set: ME3 #191 — Masters Edition III | Scryfall ID: bce2259a-f4cb-4130-9c7e-130980a8df38 | Oracle ID: de7839fb-7040-48ab-a6d4-d1952972943d
// PARTIAL — choosing an opponent as it enters is not in the engine; it does
// nothing.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLACK_VISE,
    oracle_id = "de7839fb-7040-48ab-a6d4-d1952972943d",
    scryfall_id = "bce2259a-f4cb-4130-9c7e-130980a8df38",
    coverage = Coverage::Partial(
        "choosing an opponent as it enters is not in the engine; it does nothing"
    ),
    faces = &[face!(
        name = "Black Vise",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        // NOT SUPPORTED: As this artifact enters, choose an opponent.
        // NOT SUPPORTED: At the beginning of the chosen player's upkeep, this artifact
        // deals X damage to that player, where X is the number of cards in their hand
        // minus 4.
    ],
);
