//! The Gold Saucer — (no cost) — Land — Town
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Flip a coin. If you win the flip, create a Treasure token.
//! Oracle: {3}, {T}, Sacrifice two artifacts: Draw a card.
//! Set: FIN #279 — Final Fantasy | Scryfall ID: 5363c881-443d-43df-afd8-f81e1a1741a2 | Oracle ID: 93e38650-ce22-4ab9-b79d-cc7b6477c075
// PARTIAL — the "{T}: Add {C}" mana ability is built; the two other printed
// abilities have no vocabulary, and both are named in the NOT SUPPORTED
// comments beside the ability list.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_GOLD_SAUCER,
    oracle_id = "93e38650-ce22-4ab9-b79d-cc7b6477c075",
    scryfall_id = "5363c881-443d-43df-afd8-f81e1a1741a2",
    faces = &[face!(
        name = "The Gold Saucer",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::TOWN],
    ),],
    coverage = Coverage::Partial(
        "no effect flips a coin; CostPart::Sacrifice names one permanent and carries no count"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{2}, {T}: Flip a coin. If you win the flip, create
        // a Treasure token." — nothing in the vocabulary flips a coin; no
        // Effect produces a random outcome.
        // NOT SUPPORTED: "{3}, {T}, Sacrifice two artifacts: Draw a card."
        // — CostPart::Sacrifice names one permanent and has no count, so a
        // cost that sacrifices two permanents cannot be written.
    ],
);
