//! Soldevi Sentry — {1} — Artifact Creature — Soldier
//! Oracle: {1}: Choose target opponent. Regenerate this creature. When it regenerates this way, that player may draw a card.
//! Set: ALL #132a — Alliances | Scryfall ID: 85976b5c-4eed-4cf9-b2b0-a8421a97ab2a | Oracle ID: d2a61132-d881-4c80-92f1-a151d835832a

// PARTIAL — a vanilla 1/1: the {1} ability is not played.
// NOT SUPPORTED: "{1}: Choose target opponent. Regenerate this creature. When it regenerates this way, that player may draw a card." Nothing triggers when a regeneration shield is used, and without the opponent's card (the price of the shield) the regeneration would play stronger than printed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOLDEVI_SENTRY,
    oracle_id = "d2a61132-d881-4c80-92f1-a151d835832a",
    scryfall_id = "85976b5c-4eed-4cf9-b2b0-a8421a97ab2a",
    faces = &[face!(
        name = "Soldevi Sentry",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "The {1} ability is not played: its drawback \"When it regenerates this way, that player may draw a card\" cannot be said (nothing triggers when a regeneration shield is used), and regeneration without it would play stronger than printed; Soldevi Sentry enters as a 1/1 with no abilities."
    ),
);
