//! Eternal Dragon — {5}{W}{W} — Creature — Dragon Spirit
//! Oracle: Flying
//! Oracle: {3}{W}{W}: Return this card from your graveyard to your hand. Activate only during your upkeep.
//! Oracle: Plainscycling {2} ({2}, Discard this card: Search your library for a Plains card, reveal it, put it into your hand, then shuffle.)
//! Set: AFC #66 — Forgotten Realms Commander | Scryfall ID: 0f57fc0c-cab1-4eb0-a58a-0c7a762f1572 | Oracle ID: 04d8615c-3883-4251-9790-1d8a4a40e142
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ETERNAL_DRAGON,
    oracle_id = "04d8615c-3883-4251-9790-1d8a4a40e142",
    scryfall_id = "0f57fc0c-cab1-4eb0-a58a-0c7a762f1572",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Eternal Dragon",
        mana_cost = mana!("{5}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON, subtypes::creature::SPIRIT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
