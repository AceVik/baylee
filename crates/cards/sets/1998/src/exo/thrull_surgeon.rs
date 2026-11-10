//! Thrull Surgeon — {1}{B} — Creature — Thrull
//! Oracle: {1}{B}, Sacrifice this creature: Look at target player's hand and choose a card from it. That player discards that card. Activate only as a sorcery.
//! Set: TPR #121 — Tempest Remastered | Scryfall ID: 20fffe54-4c9e-454d-bc2a-cb1fb48a6f49 | Oracle ID: ead5a2bf-f48a-43a2-89bb-df3cc587c843
// PARTIAL — Privately looking at the targeted hand and choosing a discard without revealing it publicly is unsupported; the activated ability is omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Privately looking at the targeted hand and choosing a discard without revealing it publicly is unsupported; the activated ability is omitted.
card!(
    index = index::THRULL_SURGEON,
    oracle_id = "ead5a2bf-f48a-43a2-89bb-df3cc587c843",
    scryfall_id = "20fffe54-4c9e-454d-bc2a-cb1fb48a6f49",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Privately looking at the targeted hand and choosing a discard without revealing it publicly is unsupported; the activated ability is omitted."
    ),
    faces = &[face!(
        name = "Thrull Surgeon",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THRULL],
        power = Some(1),
        toughness = Some(1),
    ),],
);
