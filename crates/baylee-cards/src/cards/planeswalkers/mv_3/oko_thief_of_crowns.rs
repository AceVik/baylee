//! Oko, Thief of Crowns — {1}{G}{U} — Legendary Planeswalker — Oko
//! Oracle: +2: Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
//! Oracle: +1: Target artifact or creature loses all abilities and becomes a green Elk creature with base power and toughness 3/3.
//! Oracle: −5: Exchange control of target artifact or creature you control and target creature an opponent controls with power 3 or less.
//! Set: ELD #197 — Throne of Eldraine | Scryfall ID: 3462a3d0-5552-49fa-9eb7-100960c55891 | Oracle ID: 60c60923-ff1b-43f7-8768-731499fcffc9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OKO_THIEF_OF_CROWNS,
    oracle_id = "60c60923-ff1b-43f7-8768-731499fcffc9",
    scryfall_id = "3462a3d0-5552-49fa-9eb7-100960c55891",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Oko, Thief of Crowns",
        mana_cost = mana!("{1}{G}{U}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::OKO],
        loyalty = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
