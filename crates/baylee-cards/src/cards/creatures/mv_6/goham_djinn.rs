//! Goham Djinn — {5}{B} — Creature — Djinn
//! Oracle: {1}{B}: Regenerate this creature.
//! Oracle: This creature gets -2/-2 as long as black is the most common color among all permanents or is tied for most common.
//! Set: INV #107 — Invasion | Scryfall ID: d67796c7-4d93-4c50-8839-bb69e075bc42 | Oracle ID: e9db02fb-78e2-44b3-9e1b-ef0148234337
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOHAM_DJINN,
    oracle_id = "e9db02fb-78e2-44b3-9e1b-ef0148234337",
    scryfall_id = "d67796c7-4d93-4c50-8839-bb69e075bc42",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Goham Djinn",
        mana_cost = mana!("{5}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
