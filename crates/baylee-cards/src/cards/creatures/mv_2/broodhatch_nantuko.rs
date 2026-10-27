//! Broodhatch Nantuko — {1}{G} — Creature — Insect Druid
//! Oracle: Whenever this creature is dealt damage, you may create that many 1/1 green Insect creature tokens.
//! Oracle: Morph {2}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: MKC #167 — Murders at Karlov Manor Commander | Scryfall ID: 73788eac-a95f-4492-b1aa-de57125cde8f | Oracle ID: b0a54050-3493-4ff6-8eca-e43b92c1f3d3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BROODHATCH_NANTUKO,
    oracle_id = "b0a54050-3493-4ff6-8eca-e43b92c1f3d3",
    scryfall_id = "73788eac-a95f-4492-b1aa-de57125cde8f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Broodhatch Nantuko",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
