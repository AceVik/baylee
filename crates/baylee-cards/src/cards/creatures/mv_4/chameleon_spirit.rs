//! Chameleon Spirit — {3}{U} — Creature — Illusion Spirit
//! Oracle: As this creature enters, choose a color.
//! Oracle: Chameleon Spirit's power and toughness are each equal to the number of permanents of the chosen color your opponents control.
//! Set: MMQ #65 — Mercadian Masques | Scryfall ID: 05972ea2-b0bc-40fd-bce4-07eebdb150d5 | Oracle ID: 9ae547df-d280-41fb-81b4-24e52d60c25b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHAMELEON_SPIRIT,
    oracle_id = "9ae547df-d280-41fb-81b4-24e52d60c25b",
    scryfall_id = "05972ea2-b0bc-40fd-bce4-07eebdb150d5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Chameleon Spirit",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION, subtypes::creature::SPIRIT],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
