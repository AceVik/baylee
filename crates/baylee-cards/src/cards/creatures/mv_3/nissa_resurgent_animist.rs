//! Nissa, Resurgent Animist — {2}{G} — Legendary Creature — Elf Scout
//! Oracle: Landfall — Whenever a land you control enters, add one mana of any color. Then if this is the second time this ability has resolved this turn, reveal cards from the top of your library until you reveal an Elf or Elemental card. Put that card into your hand and the rest on the bottom of your library in a random order.
//! Set: MAT #22 — March of the Machine: The Aftermath | Scryfall ID: 248c76d3-b5cb-4582-be17-7cd1d0cb0f58 | Oracle ID: c1fc5923-c3cd-448a-98d1-c154661c2812
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NISSA_RESURGENT_ANIMIST,
    oracle_id = "c1fc5923-c3cd-448a-98d1-c154661c2812",
    scryfall_id = "248c76d3-b5cb-4582-be17-7cd1d0cb0f58",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Nissa, Resurgent Animist",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SCOUT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
