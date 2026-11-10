//! Polar Kraken — {8}{U}{U}{U} — Creature — Kraken
//! Oracle: Trample
//! Oracle: This creature enters tapped.
//! Oracle: Cumulative upkeep—Sacrifice a land. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Set: ME1 #44 — Masters Edition | Scryfall ID: c4643512-6071-45c4-bffc-8feec5bb094f | Oracle ID: d0ff30c8-ddb7-439d-b14d-c83fe3c8da87
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::POLAR_KRAKEN,
    oracle_id = "d0ff30c8-ddb7-439d-b14d-c83fe3c8da87",
    scryfall_id = "c4643512-6071-45c4-bffc-8feec5bb094f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Polar Kraken",
        mana_cost = mana!("{8}{U}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KRAKEN],
        power = Some(11),
        toughness = Some(11),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
