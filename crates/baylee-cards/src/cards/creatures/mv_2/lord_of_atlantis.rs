//! Lord of Atlantis — {U}{U} — Creature — Merfolk
//! Oracle: Other Merfolk get +1/+1 and have islandwalk. (They can't be blocked as long as defending player controls an Island.)
//! Set: TSB #24 — Time Spiral Timeshifted | Scryfall ID: a9407b60-8921-4531-bdbe-9a82aaa38d28 | Oracle ID: cc7f290f-ca00-4285-9bdb-4b4402444f30
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LORD_OF_ATLANTIS,
    oracle_id = "cc7f290f-ca00-4285-9bdb-4b4402444f30",
    scryfall_id = "a9407b60-8921-4531-bdbe-9a82aaa38d28",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Lord of Atlantis",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
