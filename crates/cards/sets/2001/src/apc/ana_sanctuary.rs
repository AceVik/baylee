//! Ana Sanctuary — {2}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, if you control a blue or black permanent, target creature gets +1/+1 until end of turn. If you control a blue permanent and a black permanent, that creature gets +5/+5 until end of turn instead.
//! Set: APC #74 — Apocalypse | Scryfall ID: 9d1599bb-4f43-4ab3-985a-8be5219f2195 | Oracle ID: 51f17fe1-1cf7-4362-b9a2-8ec225d41b03
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANA_SANCTUARY,
    oracle_id = "51f17fe1-1cf7-4362-b9a2-8ec225d41b03",
    scryfall_id = "9d1599bb-4f43-4ab3-985a-8be5219f2195",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Ana Sanctuary",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
