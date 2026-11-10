//! Primeval Titan — {4}{G}{G} — Creature — Giant
//! Oracle: Trample
//! Oracle: Whenever this creature enters or attacks, you may search your library for up to two land cards, put them onto the battlefield tapped, then shuffle.
//! Set: IMA #183 — Iconic Masters | Scryfall ID: 6d5537da-112e-4679-a113-b5d7ce32a66b | Oracle ID: ae83ef2c-960f-4c5b-97cc-52465c687c18
// IMPLEMENTED — trample, plus an enters-or-attacks trigger that searches up
// to two land cards onto the battlefield tapped ("optional" covers both the
// "you may" and the "up to two"; the shuffle is derived).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SEARCH_TWO_LANDS: &[Effect] = &[Effect::SearchLibrary {
    filter: &Filter::LAND,
    finds: &[Find::BATTLEFIELD_TAPPED, Find::BATTLEFIELD_TAPPED],
    optional: true,
}];

card!(
    index = index::PRIMEVAL_TITAN,
    oracle_id = "ae83ef2c-960f-4c5b-97cc-52465c687c18",
    scryfall_id = "6d5537da-112e-4679-a113-b5d7ce32a66b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Primeval Titan",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(6),
        toughness = Some(6),
    ),],
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(Trigger::ETB, SEARCH_TWO_LANDS),
        triggered!(Trigger::Attacks(&Filter::This), SEARCH_TWO_LANDS),
    ],
);
