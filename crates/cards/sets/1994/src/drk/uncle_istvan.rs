//! Uncle Istvan — {1}{B}{B}{B} — Creature — Human
//! Oracle: Prevent all damage that would be dealt to this creature by creatures.
//! Set: TSB #51 — Time Spiral Timeshifted | Scryfall ID: 9bf85d06-9efe-488d-9af0-fb945b346524 | Oracle ID: 44e855a4-95f9-4515-a868-a8ad7a2db751

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UNCLE_ISTVAN,
    oracle_id = "44e855a4-95f9-4515-a868-a8ad7a2db751",
    scryfall_id = "9bf85d06-9efe-488d-9af0-fb945b346524",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Uncle Istvan",
        mana_cost = mana!("{1}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::PreventDamageFrom(&Filter::CREATURE),
    )],
);
