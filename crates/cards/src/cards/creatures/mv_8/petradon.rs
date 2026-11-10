//! Petradon — {6}{R}{R} — Creature — Nightmare Beast
//! Oracle: When this creature enters, exile two target lands.
//! Oracle: When this creature leaves the battlefield, return the exiled cards to the battlefield under their owners' control.
//! Oracle: {R}: This creature gets +1/+0 until end of turn.
//! Set: TOR #108 — Torment | Scryfall ID: 75ac6311-8516-4db2-8c1f-626f0db0d36f | Oracle ID: ef2e711f-7362-4eed-8f91-f3d653e7f434
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PETRADON,
    oracle_id = "ef2e711f-7362-4eed-8f91-f3d653e7f434",
    scryfall_id = "75ac6311-8516-4db2-8c1f-626f0db0d36f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Petradon",
        mana_cost = mana!("{6}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::BEAST],
        power = Some(5),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
