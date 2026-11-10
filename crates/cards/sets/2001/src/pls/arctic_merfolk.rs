//! Arctic Merfolk — {1}{U} — Creature — Merfolk
//! Oracle: Kicker—Return a creature you control to its owner's hand. (You may return a creature you control to its owner's hand in addition to any other costs as you cast this spell.)
//! Oracle: If this creature was kicked, it enters with a +1/+1 counter on it.
//! Set: PLS #21 — Planeshift | Scryfall ID: 86369fe5-d86d-4f4c-8f3d-dedc174f2032 | Oracle ID: c56671e1-28a6-40c3-bad5-b5e617ddb7da
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARCTIC_MERFOLK,
    oracle_id = "c56671e1-28a6-40c3-bad5-b5e617ddb7da",
    scryfall_id = "86369fe5-d86d-4f4c-8f3d-dedc174f2032",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Arctic Merfolk",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
