//! Harrow — {2}{G} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a land.
//! Oracle: Search your library for up to two basic land cards, put them onto the battlefield, then shuffle.
//! Set: EOC #98 — Edge of Eternities Commander | Scryfall ID: 3e76be46-cf81-4fef-bc5b-f87666cb0413 | Oracle ID: 705509e9-a034-4a5a-9c65-66f58748b8a2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HARROW,
    oracle_id = "705509e9-a034-4a5a-9c65-66f58748b8a2",
    scryfall_id = "3e76be46-cf81-4fef-bc5b-f87666cb0413",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Harrow",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::LAND)],
    ),],
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::BASIC_LAND,
        finds: &[Find::BATTLEFIELD, Find::BATTLEFIELD],
        optional: true,
    }])],
);
