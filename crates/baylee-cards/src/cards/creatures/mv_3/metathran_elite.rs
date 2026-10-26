//! Metathran Elite — {1}{U}{U} — Creature — Metathran Soldier
//! Oracle: This creature can't be blocked as long as it's enchanted.
//! Set: UDS #38 — Urza's Destiny | Scryfall ID: aa941f17-1b81-4017-90ae-4466eba8da2f | Oracle ID: c8a1b3dd-bc36-4d97-8f93-1151446f04e2
// IMPLEMENTED — 2/3 Metathran Soldier that can't be blocked as long as it's enchanted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::METATHRAN_ELITE,
    oracle_id = "c8a1b3dd-bc36-4d97-8f93-1151446f04e2",
    scryfall_id = "aa941f17-1b81-4017-90ae-4466eba8da2f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Metathran Elite",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::METATHRAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::This, Filter::Enchanted]),
        Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)
    )],
);
