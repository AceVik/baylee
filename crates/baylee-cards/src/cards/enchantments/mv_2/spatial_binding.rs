//! Spatial Binding — {U}{B} — Enchantment
//! Oracle: Pay 1 life: Until your next upkeep, target permanent can't phase out.
//! Set: MIR #284 — Mirage | Scryfall ID: 176d625f-1410-4ad6-a279-9a184fac6507 | Oracle ID: 52fc8217-5a14-40b5-a8fc-b6fc45af5f33
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPATIAL_BINDING,
    oracle_id = "52fc8217-5a14-40b5-a8fc-b6fc45af5f33",
    scryfall_id = "176d625f-1410-4ad6-a279-9a184fac6507",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Spatial Binding",
        mana_cost = mana!("{U}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
