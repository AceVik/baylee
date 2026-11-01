//! Cantivore — {1}{W}{W} — Creature — Lhurgoyf
//! Oracle: Vigilance
//! Oracle: Cantivore's power and toughness are each equal to the number of enchantment cards in all graveyards.
//! Set: ODY #13 — Odyssey | Scryfall ID: b5243fc3-176b-44a3-9f1a-ab069a08757a | Oracle ID: 2ed5083e-1c58-48c0-8413-390971021d94
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CANTIVORE,
    oracle_id = "2ed5083e-1c58-48c0-8413-390971021d94",
    scryfall_id = "b5243fc3-176b-44a3-9f1a-ab069a08757a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Cantivore",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LHURGOYF],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
