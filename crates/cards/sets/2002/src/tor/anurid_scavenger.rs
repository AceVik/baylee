//! Anurid Scavenger — {2}{G} — Creature — Frog Beast
//! Oracle: Protection from black
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you put a card from your graveyard on the bottom of your library.
//! Set: TOR #119 — Torment | Scryfall ID: 21a21190-3c05-40fe-9310-493ed0f9e42e | Oracle ID: 8d4d70c4-0490-4239-ac8f-dd1d1a008f08
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANURID_SCAVENGER,
    oracle_id = "8d4d70c4-0490-4239-ac8f-dd1d1a008f08",
    scryfall_id = "21a21190-3c05-40fe-9310-493ed0f9e42e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Anurid Scavenger",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FROG, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
