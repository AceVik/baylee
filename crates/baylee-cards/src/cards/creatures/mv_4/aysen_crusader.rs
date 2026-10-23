//! Aysen Crusader — {2}{W}{W} — Creature — Human Knight
//! Oracle: Aysen Crusader's power and toughness are each equal to 2 plus the number of Soldiers and Warriors you control.
//! Set: ME2 #7 — Masters Edition II | Scryfall ID: b865abb2-6bfe-4368-920b-a2c327fa6e43 | Oracle ID: 3ebbe05c-7cc4-4845-87e5-ab93d7ac9623
// IMPLEMENTED — characteristic-defining P/T equal to 2 plus Soldiers and Warriors you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SOLDIER_OR_WARRIOR: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::creature::SOLDIER),
    Filter::HasSubtype(subtypes::creature::WARRIOR),
]);
static SOLDIERS_AND_WARRIORS_YOU_CONTROL: PtCount = PtCount::YouControl(&SOLDIER_OR_WARRIOR);

card!(
    index = index::AYSEN_CRUSADER,
    oracle_id = "3ebbe05c-7cc4-4845-87e5-ab93d7ac9623",
    scryfall_id = "b865abb2-6bfe-4368-920b-a2c327fa6e43",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aysen Crusader",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPT {
            count: PtCount::Plus {
                base: &SOLDIERS_AND_WARRIORS_YOU_CONTROL,
                offset: 2,
            },
            toughness_plus: 0,
        }
    )],
);
