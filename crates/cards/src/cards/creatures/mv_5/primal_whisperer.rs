//! Primal Whisperer — {4}{G} — Creature — Elf Soldier
//! Oracle: This creature gets +2/+2 for each face-down creature on the battlefield.
//! Oracle: Morph {3}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: LGN #135 — Legions | Scryfall ID: c777432f-7965-4ad8-8d53-93919ae767d4 | Oracle ID: dfe51046-12db-4be8-a4b4-ffa6359cac15
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PRIMAL_WHISPERER,
    oracle_id = "dfe51046-12db-4be8-a4b4-ffa6359cac15",
    scryfall_id = "c777432f-7965-4ad8-8d53-93919ae767d4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Primal Whisperer",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
