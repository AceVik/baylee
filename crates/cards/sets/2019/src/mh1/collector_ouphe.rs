//! Collector Ouphe — {1}{G} — Creature — Ouphe
//! Oracle: Activated abilities of artifacts can't be activated.
//! Set: MH1 #158 — Modern Horizons | Scryfall ID: 085107a2-c1ec-473c-81d8-23e5a7197776 | Oracle ID: 0c4bc9ea-a5fd-4f44-96a1-5448eee228c4

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COLLECTOR_OUPHE,
    oracle_id = "0c4bc9ea-a5fd-4f44-96a1-5448eee228c4",
    scryfall_id = "085107a2-c1ec-473c-81d8-23e5a7197776",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Collector Ouphe",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OUPHE],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(Filter::ARTIFACT, Modifier::CantActivate)],
);
