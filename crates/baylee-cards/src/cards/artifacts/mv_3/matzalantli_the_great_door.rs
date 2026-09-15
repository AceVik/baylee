//! Matzalantli, the Great Door // The Core — {3} — Legendary Artifact // Legendary Land
//! Oracle: {T}: Draw a card, then discard a card.
//! Oracle: {4}, {T}: Transform Matzalantli. Activate only if there are four or more permanent types among cards in your graveyard. (Artifact, battle, creature, enchantment, land, and planeswalker are permanent types.)
//! Oracle: (Transforms from Matzalantli.)
//! Oracle: Fathomless descent — {T}: Add X mana of any one color, where X is the number of permanent cards in your graveyard.
//! Set: LCI #256 — The Lost Caverns of Ixalan | Scryfall ID: b4c31b29-06ba-436d-a3d9-18f4796c39be | Oracle ID: 16182e01-22ff-4786-985d-919b47c4aa4d
//! Face: Matzalantli, the Great Door — {3} — Legendary Artifact
//! Face: The Core —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MATZALANTLI_THE_GREAT_DOOR,
    oracle_id = "16182e01-22ff-4786-985d-919b47c4aa4d",
    scryfall_id = "b4c31b29-06ba-436d-a3d9-18f4796c39be",
    faces = &[
        face!(
            name = "Matzalantli, the Great Door",
            mana_cost = mana!("{3}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "The Core",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
