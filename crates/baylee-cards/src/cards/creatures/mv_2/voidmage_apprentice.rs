//! Voidmage Apprentice — {1}{U} — Creature — Human Wizard
//! Oracle: Morph {2}{U}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, counter target spell.
//! Set: JVC #4 — Duel Decks Anthology: Jace vs. Chandra | Scryfall ID: fdc000af-a161-4803-bd24-4fdde083caec | Oracle ID: 980939db-d867-4c16-a288-52f6528f2376
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VOIDMAGE_APPRENTICE,
    oracle_id = "980939db-d867-4c16-a288-52f6528f2376",
    scryfall_id = "fdc000af-a161-4803-bd24-4fdde083caec",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Voidmage Apprentice",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
