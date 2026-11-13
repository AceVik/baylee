//! Crazed Firecat — {5}{R}{R} — Creature — Elemental Cat
//! Oracle: When this creature enters, flip a coin until you lose a flip. Put a +1/+1 counter on this creature for each flip you won.
//! Set: TOR #94 — Torment | Scryfall ID: 5ef809de-ff64-4832-95dd-2ebda5942df8 | Oracle ID: ba7cc6be-e575-4592-ada7-bbad3ae2225b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRAZED_FIRECAT,
    oracle_id = "ba7cc6be-e575-4592-ada7-bbad3ae2225b",
    scryfall_id = "5ef809de-ff64-4832-95dd-2ebda5942df8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Crazed Firecat",
        mana_cost = mana!("{5}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::CAT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
