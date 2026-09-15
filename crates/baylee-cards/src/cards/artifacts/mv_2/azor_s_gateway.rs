//! Azor's Gateway // Sanctum of the Sun — {2} — Legendary Artifact // Legendary Land
//! Oracle: {1}, {T}: Draw a card, then exile a card from your hand. If cards with five or more different mana values are exiled with Azor's Gateway, you gain 5 life, untap Azor's Gateway, and transform it.
//! Oracle: (Transforms from Azor's Gateway.)
//! Oracle: {T}: Add X mana of any one color, where X is your life total.
//! Set: RIX #176 — Rivals of Ixalan | Scryfall ID: 303d51ab-b9c4-4647-950f-291daabe7b81 | Oracle ID: c0cbb347-b060-43ce-a9c5-8c835be3cf1b
//! Face: Azor's Gateway — {2} — Legendary Artifact
//! Face: Sanctum of the Sun —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = 17365,
    oracle_id = "c0cbb347-b060-43ce-a9c5-8c835be3cf1b",
    scryfall_id = "303d51ab-b9c4-4647-950f-291daabe7b81",
    faces = &[
        face!(
            name = "Azor's Gateway",
            mana_cost = mana!("{2}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Sanctum of the Sun",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
