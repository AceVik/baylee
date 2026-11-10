//! High Tide — {U} — Instant
//! Oracle: Until end of turn, whenever a player taps an Island for mana, that player adds an additional {U}.
//! Set: DMR #54 — Dominaria Remastered | Scryfall ID: babd2248-5517-4b7b-a159-9a1f7b5583c8 | Oracle ID: dc671205-f2fa-454f-9957-921a6069ad53
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ISLAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::ISLAND)]);

card!(
    index = index::HIGH_TIDE,
    oracle_id = "dc671205-f2fa-454f-9957-921a6069ad53",
    scryfall_id = "babd2248-5517-4b7b-a159-9a1f7b5583c8",
    coverage = Coverage::Implemented,
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "High Tide",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::WheneverTappedForManaThisTurn {
        by: PlayerRel::EachPlayer,
        filter: &ISLAND,
        effects: &[Effect::AddManaFor {
            who: PlayerRel::EventPlayer,
            color: ManaColor::Blue,
            amount: 1,
        }],
    }]),],
);
