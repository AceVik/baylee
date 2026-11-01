//! Gamekeeper — {3}{G} — Creature — Elf
//! Oracle: When this creature dies, you may exile it. If you do, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield and put all other cards revealed this way into your graveyard.
//! Set: DMR #162 — Dominaria Remastered | Scryfall ID: 1dba14c0-18c0-4ded-ae56-5a689106f1a1 | Oracle ID: c6921cfe-48f9-4378-b145-d9b430c2aaf6
// PARTIAL — exiling the source as a prerequisite and revealing until a creature with the remainder going to the graveyard is unsupported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GAMEKEEPER,
    oracle_id = "c6921cfe-48f9-4378-b145-d9b430c2aaf6",
    scryfall_id = "1dba14c0-18c0-4ded-ae56-5a689106f1a1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "exiling the source as a prerequisite and revealing until a creature with the remainder going to the graveyard is unsupported"
    ),
    faces = &[face!(
        name = "Gamekeeper",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: exiling the source as a prerequisite and revealing until a creature with the remainder going to the graveyard is unsupported.
);
