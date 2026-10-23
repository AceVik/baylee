//! Coat of Arms — {5} — Artifact
//! Oracle: Each creature gets +1/+1 for each other creature on the battlefield that shares at least one creature type with it. (For example, if two Goblin Warriors and a Goblin Shaman are on the battlefield, each gets +2/+2.)
//! Set: DDS #58 — Duel Decks: Mind vs. Might | Scryfall ID: 513d4c36-6ad4-4ee9-b161-3136eb59504f | Oracle ID: 5f7f133e-58ea-41ab-b1be-be4b400fac4c

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COAT_OF_ARMS,
    oracle_id = "5f7f133e-58ea-41ab-b1be-be4b400fac4c",
    scryfall_id = "513d4c36-6ad4-4ee9-b161-3136eb59504f",
    coverage = Coverage::Partial(
        "Static +1/+1 per other creature on the battlefield sharing a creature type with each affected creature is not supported; enters as an inert artifact."
    ),
    faces = &[face!(
        name = "Coat of Arms",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[],
);
