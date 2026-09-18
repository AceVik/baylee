//! Aesi, Tyrant of Gyre Strait — {4}{G}{U} — Legendary Creature — Serpent
//! Oracle: You may play an additional land on each of your turns.
//! Oracle: Landfall — Whenever a land you control enters, you may draw a card.
//! Set: DSC #210 — Duskmourn: House of Horror Commander | Scryfall ID: 673c21f8-02b6-4ac4-b2fc-df065b4ac662 | Oracle ID: 6511f317-bd38-46d0-b800-7125a3f420da
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AESI_TYRANT_OF_GYRE_STRAIT,
    oracle_id = "6511f317-bd38-46d0-b800-7125a3f420da",
    scryfall_id = "673c21f8-02b6-4ac4-b2fc-df065b4ac662",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Aesi, Tyrant of Gyre Strait",
        mana_cost = mana!("{4}{G}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::SERPENT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
