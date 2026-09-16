//! Quirion Ranger — {G} — Creature — Elf Ranger
//! Oracle: Return a Forest you control to its owner's hand: Untap target creature. Activate only once each turn.
//! Set: MH2 #285 — Modern Horizons 2 | Scryfall ID: 320fdf89-e158-41c5-b0bf-fee9dec36a75 | Oracle ID: 3ecaefc8-ead2-47a3-a7ea-b030faab65a7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QUIRION_RANGER,
    oracle_id = "3ecaefc8-ead2-47a3-a7ea-b030faab65a7",
    scryfall_id = "320fdf89-e158-41c5-b0bf-fee9dec36a75",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Quirion Ranger",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::RANGER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
