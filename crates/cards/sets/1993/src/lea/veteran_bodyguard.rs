//! Veteran Bodyguard — {3}{W}{W} — Creature — Human
//! Oracle: As long as this creature is untapped, all damage that would be dealt to you by unblocked creatures is dealt to this creature instead.
//! Set: ME4 #32 — Masters Edition IV | Scryfall ID: 1dd4615d-7828-43fb-ab89-f7c732bca01a | Oracle ID: d29078c0-1fb8-437a-81d1-bb319f646941

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VETERAN_BODYGUARD,
    oracle_id = "d29078c0-1fb8-437a-81d1-bb319f646941",
    scryfall_id = "1dd4615d-7828-43fb-ab89-f7c732bca01a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Veteran Bodyguard",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(2),
        toughness = Some(5),
    ),],
    // "As long as this creature is untapped" is read on the permanent the
    // redirection applies to, as the damage would be dealt, so a Bodyguard
    // tapped a moment earlier in the same resolution is already out of it.
    abilities = &[static_ability!(
        Filter::And(&[Filter::This, Filter::Untapped]),
        Modifier::RedirectDamageToYou(&f!(unblocked CREATURE)),
    )],
);
