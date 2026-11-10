//! Ad Nauseam — {3}{B}{B} — Instant
//! Oracle: Reveal the top card of your library and put that card into your hand. You lose life equal to its mana value. You may repeat this process any number of times.
//! Set: 2XM #76 — Double Masters | Scryfall ID: a9f2c53e-ff58-4aa8-89a6-4f45628cc571 | Oracle ID: 981b0e21-e5e6-4a1e-bfde-679d56623f7f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AD_NAUSEAM,
    oracle_id = "981b0e21-e5e6-4a1e-bfde-679d56623f7f",
    scryfall_id = "a9f2c53e-ff58-4aa8-89a6-4f45628cc571",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Ad Nauseam",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
