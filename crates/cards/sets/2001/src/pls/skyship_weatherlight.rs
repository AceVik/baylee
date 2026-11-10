//! Skyship Weatherlight — {4} — Legendary Artifact
//! Oracle: When Skyship Weatherlight enters, search your library for any number of artifact and/or creature cards, exile them, then shuffle.
//! Oracle: {4}, {T}: Choose a card at random that was exiled with Skyship Weatherlight. Put that card into its owner's hand.
//! Set: PLS #133 — Planeshift | Scryfall ID: 63f5498b-bb12-48ec-811b-b52e45ffddaf | Oracle ID: 57889a8e-0039-45c2-a608-ab315d51aa3a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SKYSHIP_WEATHERLIGHT,
    oracle_id = "57889a8e-0039-45c2-a608-ab315d51aa3a",
    scryfall_id = "63f5498b-bb12-48ec-811b-b52e45ffddaf",
    faces = &[face!(
        name = "Skyship Weatherlight",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
