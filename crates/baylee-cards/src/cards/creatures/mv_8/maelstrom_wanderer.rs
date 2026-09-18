//! Maelstrom Wanderer — {5}{G}{U}{R} — Legendary Creature — Elemental
//! Oracle: Creatures you control have haste.
//! Oracle: Cascade, cascade (When you cast this spell, exile cards from the top of your library until you exile a nonland card that costs less. You may cast it without paying its mana cost. Put the exiled cards on the bottom in a random order. Then do it again.)
//! Set: ECC #127 — Lorwyn Eclipsed Commander | Scryfall ID: 5685b28a-b943-4143-b9ed-4796c1ffbf9c | Oracle ID: ad9b7fbc-61c8-43ee-a65c-99206fd1e4df
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MAELSTROM_WANDERER,
    oracle_id = "ad9b7fbc-61c8-43ee-a65c-99206fd1e4df",
    scryfall_id = "5685b28a-b943-4143-b9ed-4796c1ffbf9c",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Maelstrom Wanderer",
        mana_cost = mana!("{5}{G}{U}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(7),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
