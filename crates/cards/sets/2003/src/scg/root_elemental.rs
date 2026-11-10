//! Root Elemental — {4}{G}{G} — Creature — Elemental
//! Oracle: Morph {5}{G}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, you may put a creature card from your hand onto the battlefield.
//! Set: MKC #182 — Murders at Karlov Manor Commander | Scryfall ID: e8d6e393-7378-4113-a5ff-44e9518eff76 | Oracle ID: 182d38b4-48bb-4050-a19a-c0a6bb241bbd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROOT_ELEMENTAL,
    oracle_id = "182d38b4-48bb-4050-a19a-c0a6bb241bbd",
    scryfall_id = "e8d6e393-7378-4113-a5ff-44e9518eff76",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Root Elemental",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(6),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
