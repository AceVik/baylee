//! Recruiter of the Guard — {2}{W} — Creature — Human Soldier
//! Oracle: When this creature enters, you may search your library for a creature card with toughness 2 or less, reveal it, put it into your hand, then shuffle.
//! Set: CN2 #90 — Conspiracy: Take the Crown | Scryfall ID: 8e4c6ba1-1abc-478f-9b7c-97e9e3c92fb0 | Oracle ID: d521a329-a53a-4962-810a-2abed80df260
// IMPLEMENTED — ETB tutor with the real toughness filter
// (Filter::ToughnessAtMost).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static TOUGH_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::ToughnessAtMost(2)]);

card! {
    index: 126,
    oracle_id: "d521a329-a53a-4962-810a-2abed80df260",
    scryfall_id: "8e4c6ba1-1abc-478f-9b7c-97e9e3c92fb0",
    faces: &[face! {
        name: "Recruiter of the Guard",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::SOLDIER],
        power: Some(1),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::SearchLibrary {
            filter: &TOUGH_CREATURE,
            finds: &[Find::HAND],
            optional: true,
        }])],
}
