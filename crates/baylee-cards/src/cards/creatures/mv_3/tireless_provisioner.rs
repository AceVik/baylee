//! Tireless Provisioner — {2}{G} — Creature — Elf Scout
//! Oracle: Landfall — Whenever a land you control enters, create a Food token or a Treasure token. (Food is an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life." Treasure is an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
//! Set: MOC #313 — March of the Machine Commander | Scryfall ID: a1e048e0-19d2-4076-892d-f8b3104dee37 | Oracle ID: ab8d5f5c-1976-4f77-8ed2-8d28ee666741
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TIRELESS_PROVISIONER,
    oracle_id = "ab8d5f5c-1976-4f77-8ed2-8d28ee666741",
    scryfall_id = "a1e048e0-19d2-4076-892d-f8b3104dee37",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Tireless Provisioner",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SCOUT],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
