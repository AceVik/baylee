//! Omen of Fire — {3}{R}{R} — Instant
//! Oracle: Return all Islands to their owners' hands. Each player sacrifices a Plains or a white permanent of their choice for each white permanent they control.
//! Set: ALL #75 — Alliances | Scryfall ID: 9c724b46-6e17-4bee-9bc6-e9fc5a379dd7 | Oracle ID: 5935e9b3-b9b9-413c-ba17-103f5792131a
// PARTIAL — only the Islands are returned; the sacrifice sentence is not played.
// NOT SUPPORTED: "Each player sacrifices a Plains or a white permanent of their choice for each white permanent they control." The count is per player and read from that player's white permanents (a player with a Plains and no white permanent sacrifices nothing), and no sacrifice effect takes a per-player count, so the sentence is left off.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLANDS: Filter = Filter::HasSubtype(subtypes::land::ISLAND);

card!(
    index = index::OMEN_OF_FIRE,
    oracle_id = "5935e9b3-b9b9-413c-ba17-103f5792131a",
    scryfall_id = "9c724b46-6e17-4bee-9bc6-e9fc5a379dd7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Having each player sacrifice a Plains or a white permanent of their choice for each white permanent they control needs a per-player counted sacrifice that no effect offers (one sacrifice each would hit a player with a Plains and no white permanent); Omen of Fire returns all Islands to their owners' hands and does nothing more.",
    ),
    faces = &[face!(
        name = "Omen of Fire",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::ReturnAllToHand {
        filter: &ISLANDS,
        opponents_only: false,
    },])],
);
