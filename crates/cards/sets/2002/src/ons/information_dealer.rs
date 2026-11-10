//! Information Dealer — {1}{U} — Creature — Human Wizard
//! Oracle: {T}: Look at the top X cards of your library, where X is the number of Wizards on the battlefield, then put them back in any order.
//! Set: ONS #88 — Onslaught | Scryfall ID: a45ac59c-654d-44de-b266-532d44b34137 | Oracle ID: 52d10e81-c8ae-44bb-bb83-f4fe737f30d7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INFORMATION_DEALER,
    oracle_id = "52d10e81-c8ae-44bb-bb83-f4fe737f30d7",
    scryfall_id = "a45ac59c-654d-44de-b266-532d44b34137",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Information Dealer",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
