//! Gilded Drake — {1}{U} — Creature — Drake
//! Oracle: Flying
//! Oracle: When this creature enters, exchange control of this creature and up to one target creature an opponent controls. If you don't or can't make an exchange, sacrifice this creature. This ability still resolves if its target becomes illegal.
//! Set: USG #76 — Urza's Saga | Scryfall ID: 8de3fdae-cc2c-4a14-b15b-4fe1a983dfbf | Oracle ID: 7f06c098-6482-4bf3-a9a1-110d6d5b5703
// IMPLEMENTED — control exchange with sacrifice fallback.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static OPPONENT_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent]);

card! {
    index: 58,
    oracle_id: "7f06c098-6482-4bf3-a9a1-110d6d5b5703",
    scryfall_id: "8de3fdae-cc2c-4a14-b15b-4fe1a983dfbf",
    faces: &[face! {
        name: "Gilded Drake",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::DRAKE],
        power: Some(2),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLYING,
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::ExchangeControlOrSacrifice], targets: Some(TargetReq {
            spec: TargetSpec::Object(&OPPONENT_CREATURE),
            min: 0,
            max: 1,
            count_is_x: false,
        }))],
}
