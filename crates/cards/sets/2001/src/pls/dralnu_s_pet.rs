//! Dralnu's Pet — {1}{U}{U} — Creature — Shapeshifter
//! Oracle: Kicker—{2}{B}, Discard a creature card. (You may pay {2}{B} and discard a creature card in addition to any other costs as you cast this spell.)
//! Oracle: If this creature was kicked, it enters with flying and with X +1/+1 counters on it, where X is the discarded card's mana value.
//! Set: PLS #23 — Planeshift | Scryfall ID: cd5f4daf-7b54-4425-a93a-19532dfb83ca | Oracle ID: ee626bd3-c18b-4d26-a9bd-8c23bdc02090
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRALNU_S_PET,
    oracle_id = "ee626bd3-c18b-4d26-a9bd-8c23bdc02090",
    scryfall_id = "cd5f4daf-7b54-4425-a93a-19532dfb83ca",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Dralnu's Pet",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
