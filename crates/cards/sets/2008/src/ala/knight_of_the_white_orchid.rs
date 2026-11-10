//! Knight of the White Orchid — {W}{W} — Creature — Human Knight
//! Oracle: First strike
//! Oracle: When this creature enters, if an opponent controls more lands than you, you may search your library for a Plains card, put it onto the battlefield, then shuffle.
//! Set: MOC #193 — March of the Machine Commander | Scryfall ID: f332cd21-46ed-4fff-9fd9-d5975bf0004d | Oracle ID: 7d0efc64-151f-45b4-8c0b-c32609f9d862
// IMPLEMENTED — first strike and conditional optional Plains search.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PLAINS: Filter = Filter::HasSubtype(subtypes::land::PLAINS);

card!(
    index = index::KNIGHT_OF_THE_WHITE_ORCHID,
    oracle_id = "7d0efc64-151f-45b4-8c0b-c32609f9d862",
    scryfall_id = "f332cd21-46ed-4fff-9fd9-d5975bf0004d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Knight of the White Orchid",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::MayDo {
            effects: &[Effect::SearchLibrary {
                filter: &PLAINS,
                finds: &[Find::BATTLEFIELD],
                optional: true,
            }],
        }],
        condition = Some(Condition::OpponentControlsMore(&Filter::LAND)),
    )],
);
