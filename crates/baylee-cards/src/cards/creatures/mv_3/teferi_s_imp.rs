//! Teferi's Imp — {2}{U} — Creature — Imp
//! Oracle: Flying
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Oracle: Whenever this creature phases out, discard a card.
//! Oracle: Whenever this creature phases in, draw a card.
//! Set: MIR #98 — Mirage | Scryfall ID: 048f2368-0e2f-4197-b977-353d38b38ccc | Oracle ID: e4826967-b2f9-4f32-8411-d243edf41082
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TEFERI_S_IMP,
    oracle_id = "e4826967-b2f9-4f32-8411-d243edf41082",
    scryfall_id = "048f2368-0e2f-4197-b977-353d38b38ccc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Teferi's Imp",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::IMP],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
