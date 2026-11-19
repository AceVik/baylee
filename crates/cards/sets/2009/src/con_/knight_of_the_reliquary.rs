//! Knight of the Reliquary — {1}{G}{W} — Creature — Human Knight
//! Oracle: This creature gets +1/+1 for each land card in your graveyard.
//! Oracle: {T}, Sacrifice a Forest or Plains: Search your library for a land card, put it onto the battlefield, then shuffle.
//! Set: IMA #203 — Iconic Masters | Scryfall ID: 205df311-a72d-40f1-b2ea-9b30274bc9bd | Oracle ID: 393e5331-4456-4ba3-b410-bb630bb053db
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KNIGHT_OF_THE_RELIQUARY,
    oracle_id = "393e5331-4456-4ba3-b410-bb630bb053db",
    scryfall_id = "205df311-a72d-40f1-b2ea-9b30274bc9bd",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Knight of the Reliquary",
        mana_cost = mana!("{1}{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
