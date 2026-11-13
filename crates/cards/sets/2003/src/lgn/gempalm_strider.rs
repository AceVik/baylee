//! Gempalm Strider — {1}{G} — Creature — Elf
//! Oracle: Cycling {2}{G}{G} ({2}{G}{G}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, Elf creatures get +2/+2 until end of turn.
//! Set: EVG #6 — Duel Decks Anthology: Elves vs. Goblins | Scryfall ID: 574b1d7e-beee-4744-a266-bce9c93bf1ca | Oracle ID: 48792146-e20c-4d93-83af-f746069fc6f1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GEMPALM_STRIDER,
    oracle_id = "48792146-e20c-4d93-83af-f746069fc6f1",
    scryfall_id = "574b1d7e-beee-4744-a266-bce9c93bf1ca",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gempalm Strider",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
