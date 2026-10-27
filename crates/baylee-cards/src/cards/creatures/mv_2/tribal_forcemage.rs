//! Tribal Forcemage — {1}{G} — Creature — Elf Wizard
//! Oracle: Morph {1}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, creatures of the creature type of your choice get +2/+2 and gain trample until end of turn.
//! Set: LGN #142 — Legions | Scryfall ID: 104735d7-6cea-4d4a-8cc8-e1934883da97 | Oracle ID: 245934d9-c7b7-4a97-9c56-7760d934ef27
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRIBAL_FORCEMAGE,
    oracle_id = "245934d9-c7b7-4a97-9c56-7760d934ef27",
    scryfall_id = "104735d7-6cea-4d4a-8cc8-e1934883da97",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Tribal Forcemage",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
