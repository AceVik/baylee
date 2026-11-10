//! Hibernation Sliver — {U}{B} — Creature — Sliver
//! Oracle: All Slivers have "Pay 2 life: Return this permanent to its owner's hand."
//! Set: CMM #925 — Commander Masters | Scryfall ID: 1e224cfb-5bc1-490c-ab1e-f5405dc2fa0b | Oracle ID: edb8a6d6-9e25-4048-a842-e854eece42c2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIBERNATION_SLIVER,
    oracle_id = "edb8a6d6-9e25-4048-a842-e854eece42c2",
    scryfall_id = "1e224cfb-5bc1-490c-ab1e-f5405dc2fa0b",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hibernation Sliver",
        mana_cost = mana!("{U}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        Filter::HasSubtype(subtypes::creature::SLIVER),
        Modifier::GrantActivated {
            cost: cost!(PayLife(2)),
            effects: &[Effect::bounce(TargetSpec::ThisObject)],
            mana_ability: false
        }
    ),],
);
