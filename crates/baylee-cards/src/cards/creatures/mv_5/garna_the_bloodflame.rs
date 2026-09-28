//! Garna, the Bloodflame — {3}{B}{R} — Legendary Creature — Human Warrior
//! Oracle: Flash
//! Oracle: When Garna enters, return to your hand all creature cards in your graveyard that were put there from anywhere this turn.
//! Oracle: Other creatures you control have haste.
//! Set: DMC #151 — Dominaria United Commander | Scryfall ID: 732797d3-59e1-4fff-bcf0-22c2606a7a5f | Oracle ID: 97cb993c-90ae-49ff-8ccb-b4fe317a43ef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GARNA_THE_BLOODFLAME,
    oracle_id = "97cb993c-90ae-49ff-8ccb-b4fe317a43ef",
    scryfall_id = "732797d3-59e1-4fff-bcf0-22c2606a7a5f",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Garna, the Bloodflame",
        mana_cost = mana!("{3}{B}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
