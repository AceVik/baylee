//! Birchlore Rangers — {G} — Creature — Elf Druid Ranger
//! Oracle: Tap two untapped Elves you control: Add one mana of any color.
//! Oracle: Morph {G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #248 — Onslaught | Scryfall ID: 8ce3a3a1-3569-4909-a604-f78d4888781e | Oracle ID: 667c5676-7a18-4fa9-b965-b26797538e23
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BIRCHLORE_RANGERS,
    oracle_id = "667c5676-7a18-4fa9-b965-b26797538e23",
    scryfall_id = "8ce3a3a1-3569-4909-a604-f78d4888781e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Birchlore Rangers",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::ELF,
            subtypes::creature::DRUID,
            subtypes::creature::RANGER
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
