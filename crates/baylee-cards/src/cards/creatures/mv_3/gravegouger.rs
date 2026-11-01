//! Gravegouger — {2}{B} — Creature — Nightmare Horror
//! Oracle: When this creature enters, exile up to two target cards from a single graveyard.
//! Oracle: When this creature leaves the battlefield, return the exiled cards to their owner's graveyard.
//! Set: TOR #62 — Torment | Scryfall ID: acfcd559-374e-4e6f-9333-2e5c855abff5 | Oracle ID: 2677c0d9-1251-478e-b157-072859c593a9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRAVEGOUGER,
    oracle_id = "2677c0d9-1251-478e-b157-072859c593a9",
    scryfall_id = "acfcd559-374e-4e6f-9333-2e5c855abff5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Gravegouger",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::HORROR],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
