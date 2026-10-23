//! Bubbling Muck — {B} — Sorcery
//! Oracle: Until end of turn, whenever a player taps a Swamp for mana, that player adds an additional {B}.
//! Set: UDS #54 — Urza's Destiny | Scryfall ID: 6ca76614-78a1-4535-9162-70469d1e8a13 | Oracle ID: 3c7f2b29-9f42-41ab-a2d4-7a450fb0242d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SWAMP: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::SWAMP)]);

card!(
    index = index::BUBBLING_MUCK,
    oracle_id = "3c7f2b29-9f42-41ab-a2d4-7a450fb0242d",
    scryfall_id = "6ca76614-78a1-4535-9162-70469d1e8a13",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bubbling Muck",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::WheneverTappedForManaThisTurn {
        by: PlayerRel::EachPlayer,
        filter: &SWAMP,
        effects: &[Effect::AddManaFor {
            who: PlayerRel::EventPlayer,
            color: ManaColor::Black,
            amount: 1,
        }],
    }])],
);
