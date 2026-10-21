//! Orcish Librarian — {1}{R} — Creature — Orc
//! Oracle: {R}, {T}: Look at the top eight cards of your library. Exile four of them at random, then put the rest on top of your library in any order.
//! Set: TSB #66 — Time Spiral Timeshifted | Scryfall ID: 31f074bb-5ff6-4ce7-a186-1782d940c4a9 | Oracle ID: 3a05904d-5439-446d-a2a1-1475c9c7371d
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: No Effect looks at the top eight cards of your library, exiles four of them at random, and puts the rest back on top in any order; plays as a vanilla 1/1.
card!(
    index = index::ORCISH_LIBRARIAN,
    oracle_id = "3a05904d-5439-446d-a2a1-1475c9c7371d",
    scryfall_id = "31f074bb-5ff6-4ce7-a186-1782d940c4a9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no effect exiles four cards at random from the top eight cards of your library and reorders the rest on top; plays as a vanilla 1/1",
    ),
    faces = &[face!(
        name = "Orcish Librarian",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC],
        power = Some(1),
        toughness = Some(1),
    ),],
);
