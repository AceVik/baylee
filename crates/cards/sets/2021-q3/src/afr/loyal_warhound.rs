//! Loyal Warhound — {1}{W} — Creature — Dog
//! Oracle: Vigilance
//! Oracle: When this creature enters, if an opponent controls more lands than you, search your library for a basic Plains card, put it onto the battlefield tapped, then shuffle.
//! Set: AFR #23 — Adventures in the Forgotten Realms | Scryfall ID: f408c5e4-e18d-4e4d-959a-f41df4c3019c | Oracle ID: cc6a83c7-e645-4a53-9550-be79b42cd851
// IMPLEMENTED — vigilance and conditional basic Plains search tapped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BASIC_PLAINS: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::HasSubtype(subtypes::land::PLAINS),
]);

card!(
    index = index::LOYAL_WARHOUND,
    oracle_id = "cc6a83c7-e645-4a53-9550-be79b42cd851",
    scryfall_id = "f408c5e4-e18d-4e4d-959a-f41df4c3019c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::VIGILANCE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Loyal Warhound",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DOG],
        power = Some(3),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::SearchLibrary {
            filter: &BASIC_PLAINS,
            finds: &[Find::BATTLEFIELD_TAPPED],
            optional: false,
        }],
        condition = Some(Condition::OpponentControlsMore(&Filter::LAND)),
    )],
);
