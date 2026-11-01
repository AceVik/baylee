//! Elvish Soultiller — {3}{G}{G} — Creature — Elf Mutant
//! Oracle: When this creature dies, choose a creature type. Shuffle all creature cards of that type from your graveyard into your library.
//! Set: LGN #124 — Legions | Scryfall ID: 9e2c8de5-bc80-4fad-af09-6d0a639f6e18 | Oracle ID: 10b0b60f-9976-41f9-855b-1786b6f86f55
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVISH_SOULTILLER,
    oracle_id = "10b0b60f-9976-41f9-855b-1786b6f86f55",
    scryfall_id = "9e2c8de5-bc80-4fad-af09-6d0a639f6e18",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Elvish Soultiller",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::MUTANT],
        power = Some(5),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
