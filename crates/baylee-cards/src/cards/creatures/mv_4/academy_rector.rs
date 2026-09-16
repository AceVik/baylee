//! Academy Rector — {3}{W} — Creature — Human Cleric
//! Oracle: When this creature dies, you may exile it. If you do, search your library for an enchantment card, put that card onto the battlefield, then shuffle.
//! Set: UDS #1 — Urza's Destiny | Scryfall ID: 4367bc78-0912-4abd-8edd-bc792558d01a | Oracle ID: e3c85068-b4b6-40b9-a16c-5c3b2d059ec4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ACADEMY_RECTOR,
    oracle_id = "e3c85068-b4b6-40b9-a16c-5c3b2d059ec4",
    scryfall_id = "4367bc78-0912-4abd-8edd-bc792558d01a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Academy Rector",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
