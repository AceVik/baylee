//! Surprise Deployment — {3}{W} — Instant
//! Oracle: Cast this spell only during combat.
//! Oracle: You may put a nonwhite creature card from your hand onto the battlefield. At the beginning of the next end step, return that creature to your hand. (Return it only if it's on the battlefield.)
//! Set: PLS #18 — Planeshift | Scryfall ID: 9a26148b-b981-4af5-995b-52b1426737e3 | Oracle ID: 61a1748c-e772-4338-9edd-720cd5d2fc12
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SURPRISE_DEPLOYMENT,
    oracle_id = "61a1748c-e772-4338-9edd-720cd5d2fc12",
    scryfall_id = "9a26148b-b981-4af5-995b-52b1426737e3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Surprise Deployment",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
