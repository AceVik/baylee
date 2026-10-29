//! Imperial Hellkite — {5}{R}{R} — Creature — Dragon
//! Oracle: Flying
//! Oracle: Morph {6}{R}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, you may search your library for a Dragon card, reveal it, put it into your hand, then shuffle.
//! Set: MKC #155 — Murders at Karlov Manor Commander | Scryfall ID: cffb1feb-9368-4d91-bd4b-36a24fe849e6 | Oracle ID: 99642419-ab24-4770-9f1c-84d2806441b5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IMPERIAL_HELLKITE,
    oracle_id = "99642419-ab24-4770-9f1c-84d2806441b5",
    scryfall_id = "cffb1feb-9368-4d91-bd4b-36a24fe849e6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Imperial Hellkite",
        mana_cost = mana!("{5}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
