//! Ertai's Familiar — {1}{U} — Creature — Illusion
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Oracle: When this creature phases out or leaves the battlefield, mill three cards.
//! Oracle: {U}: Until your next upkeep, this creature can't phase out.
//! Set: WTH #38 — Weatherlight | Scryfall ID: 354c9de7-0cdf-4302-9d1a-ae17eca13053 | Oracle ID: 711cd80d-c58d-4c2a-87db-d2e4c75f60d3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ERTAI_S_FAMILIAR,
    oracle_id = "711cd80d-c58d-4c2a-87db-d2e4c75f60d3",
    scryfall_id = "354c9de7-0cdf-4302-9d1a-ae17eca13053",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Ertai's Familiar",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
