//! Gigapede — {3}{G}{G} — Creature — Insect
//! Oracle: Shroud (This creature can't be the target of spells or abilities.)
//! Oracle: At the beginning of your upkeep, if this card is in your graveyard, you may discard a card. If you do, return this card to your hand.
//! Set: VMA #214 — Vintage Masters | Scryfall ID: 35a12eeb-905b-4514-828c-96952b148d81 | Oracle ID: 9be31785-2975-424f-aada-25ee2a0bf6d8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GIGAPEDE,
    oracle_id = "9be31785-2975-424f-aada-25ee2a0bf6d8",
    scryfall_id = "35a12eeb-905b-4514-828c-96952b148d81",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gigapede",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(6),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
