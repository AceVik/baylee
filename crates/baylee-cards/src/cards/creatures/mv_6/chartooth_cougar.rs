//! Chartooth Cougar — {5}{R} — Creature — Cat Beast
//! Oracle: {R}: This creature gets +1/+0 until end of turn.
//! Oracle: Mountaincycling {2} ({2}, Discard this card: Search your library for a Mountain card, reveal it, put it into your hand, then shuffle.)
//! Set: A25 #125 — Masters 25 | Scryfall ID: 03a32ab2-cc55-43fb-82d8-c51a3849b344 | Oracle ID: 717152c3-2082-49aa-9274-a1a6abe54b65
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHARTOOTH_COUGAR,
    oracle_id = "717152c3-2082-49aa-9274-a1a6abe54b65",
    scryfall_id = "03a32ab2-cc55-43fb-82d8-c51a3849b344",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Chartooth Cougar",
        mana_cost = mana!("{5}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAT, subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
