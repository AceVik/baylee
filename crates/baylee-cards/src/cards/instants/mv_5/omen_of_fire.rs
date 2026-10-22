//! Omen of Fire — {3}{R}{R} — Instant
//! Oracle: Return all Islands to their owners' hands. Each player sacrifices a Plains or a white permanent of their choice for each white permanent they control.
//! Set: ALL #75 — Alliances | Scryfall ID: 9c724b46-6e17-4bee-9bc6-e9fc5a379dd7 | Oracle ID: 5935e9b3-b9b9-413c-ba17-103f5792131a
// NOT SUPPORTED: having each player sacrifice a Plains or a white permanent for each white permanent they control requires a per-player counted sacrifice effect.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLANDS: Filter = Filter::HasSubtype(subtypes::land::ISLAND);
static PLAINS_OR_WHITE: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::PLAINS),
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

card!(
    index = index::OMEN_OF_FIRE,
    oracle_id = "5935e9b3-b9b9-413c-ba17-103f5792131a",
    scryfall_id = "9c724b46-6e17-4bee-9bc6-e9fc5a379dd7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Having each player sacrifice a Plains or a white permanent for each white permanent they control requires a per-player counted sacrifice effect; in play, Omen of Fire returns all Islands to their owners' hands and each player sacrifices one Plains or white permanent of their choice.",
    ),
    faces = &[face!(
        name = "Omen of Fire",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[
        Effect::ReturnAllToHand {
            filter: &ISLANDS,
            opponents_only: false,
        },
        Effect::SacrificeFilter {
            who: PlayerRel::EachPlayer,
            filter: &PLAINS_OR_WHITE,
        },
    ])],
);
