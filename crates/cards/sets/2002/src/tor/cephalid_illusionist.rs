//! Cephalid Illusionist — {1}{U} — Creature — Octopus Wizard
//! Oracle: Whenever this creature becomes the target of a spell or ability, mill three cards.
//! Oracle: {2}{U}, {T}: Prevent all combat damage that would be dealt to and dealt by target creature you control this turn.
//! Set: TOR #28 — Torment | Scryfall ID: dceb8cf5-b31a-400e-aea5-ad0c3552d697 | Oracle ID: 3c3067c4-0ae0-4c6a-9174-e3c981a20235
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CEPHALID_ILLUSIONIST,
    oracle_id = "3c3067c4-0ae0-4c6a-9174-e3c981a20235",
    scryfall_id = "dceb8cf5-b31a-400e-aea5-ad0c3552d697",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cephalid Illusionist",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OCTOPUS, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
