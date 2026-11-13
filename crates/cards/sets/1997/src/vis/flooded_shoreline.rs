//! Flooded Shoreline — {U}{U} — Enchantment
//! Oracle: {U}{U}, Return two Islands you control to their owner's hand: Return target creature to its owner's hand.
//! Set: VIS #32 — Visions | Scryfall ID: 49db9f58-380f-496e-9d3d-6776d30fb564 | Oracle ID: 01fd889f-1839-44b9-b936-0e62c8f8a730
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_ISLAND: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(subtypes::land::ISLAND),
]);

card!(
    index = index::FLOODED_SHORELINE,
    oracle_id = "01fd889f-1839-44b9-b936-0e62c8f8a730",
    scryfall_id = "49db9f58-380f-496e-9d3d-6776d30fb564",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Flooded Shoreline",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(
            "{U}{U}",
            ReturnToHand(&YOUR_ISLAND),
            ReturnToHand(&YOUR_ISLAND)
        ),
        &[Effect::ReturnToHand {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
