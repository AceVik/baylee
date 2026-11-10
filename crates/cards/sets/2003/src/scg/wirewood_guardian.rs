//! Wirewood Guardian — {5}{G}{G} — Creature — Elf Mutant
//! Oracle: Forestcycling {2} ({2}, Discard this card: Search your library for a Forest card, reveal it, put it into your hand, then shuffle.)
//! Set: SCG #132 — Scourge | Scryfall ID: e8676b1f-e37c-4ae1-9dbe-d000369fa422 | Oracle ID: 61ade140-5b11-4882-82e5-a075fc80b260
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WIREWOOD_GUARDIAN,
    oracle_id = "61ade140-5b11-4882-82e5-a075fc80b260",
    scryfall_id = "e8676b1f-e37c-4ae1-9dbe-d000369fa422",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wirewood Guardian",
        mana_cost = mana!("{5}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::MUTANT],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
