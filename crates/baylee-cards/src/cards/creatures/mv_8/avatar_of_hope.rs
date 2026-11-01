//! Avatar of Hope — {6}{W}{W} — Creature — Avatar
//! Oracle: If you have 3 or less life, this spell costs {6} less to cast.
//! Oracle: Flying
//! Oracle: This creature can block any number of creatures.
//! Set: 8ED #4 — Eighth Edition | Scryfall ID: af5d9362-4f6a-43ad-9494-9c2014a4b5ad | Oracle ID: e2f22cef-9494-4356-aa5c-619aecd7e74d
// PARTIAL — 4/9 flying that can block any number of creatures.
// NOT SUPPORTED: conditional {6} self-cost reduction when you have 3 or less life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVATAR_OF_HOPE,
    oracle_id = "e2f22cef-9494-4356-aa5c-619aecd7e74d",
    scryfall_id = "af5d9362-4f6a-43ad-9494-9c2014a4b5ad",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial("self-cost reduction when at 3 or less life is not supported",),
    faces = &[face!(
        name = "Avatar of Hope",
        mana_cost = mana!("{6}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(4),
        toughness = Some(9),
    ),],
    abilities = &[static_ability!(Filter::This, Modifier::CanBlockAnyNumber),],
);
