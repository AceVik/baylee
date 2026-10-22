//! Leering Gargoyle — {1}{W}{U} — Creature — Gargoyle
//! Oracle: Flying
//! Oracle: {T}: This creature gets -2/+2 and loses flying until end of turn.
//! Set: MIR #271 — Mirage | Scryfall ID: a05d1cac-7012-4d22-82d3-0f82b168fe68 | Oracle ID: fc0907eb-a3fc-40ef-950d-b05aee9d5da7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LEERING_GARGOYLE,
    oracle_id = "fc0907eb-a3fc-40ef-950d-b05aee9d5da7",
    scryfall_id = "a05d1cac-7012-4d22-82d3-0f82b168fe68",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Leering Gargoyle",
        mana_cost = mana!("{1}{W}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GARGOYLE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::ModifySource {
                modifier: Modifier::ModifyPT(-2, 2),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifySource {
                modifier: Modifier::RemoveKeyword(KeywordSet::FLYING),
                duration: Duration::UntilEndOfTurn,
            },
        ],
    )],
);
