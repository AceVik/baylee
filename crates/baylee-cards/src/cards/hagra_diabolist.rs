//! Hagra Diabolist — {4}{B} — Creature — Ogre Shaman Ally
//! Oracle: Whenever this creature or another Ally you control enters, you may have target player lose life equal to the number of Allies you control.
//! Set: ZEN #95 — Zendikar | Scryfall ID: c303e7e2-cb22-4dea-889f-d03e2494ed0f | Oracle ID: 5e2c1e0e-0a10-416a-9b50-96ee0cbbc24e
// IMPLEMENTED — rally life loss per Ally (opponent heads-up; target player
// choice for multiplayer is a protocol M3 item).

use crate::filters::YOUR_ALLIES;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ALLIES_YOU: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

card! {
    index: 63,
    oracle_id: "5e2c1e0e-0a10-416a-9b50-96ee0cbbc24e",
    scryfall_id: "c303e7e2-cb22-4dea-889f-d03e2494ed0f",
    faces: &[face! {
        name: "Hagra Diabolist",
        mana_cost: baylee_core::mana!("{4}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::OGRE, creature::SHAMAN, creature::ALLY],
        power: Some(3),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&YOUR_ALLIES), &[Effect::LoseLife {
            amount: Amount::CountOf {
                filter: &ALLIES_YOU,
                zone: ZoneSel::Battlefield,
            },
            target: PlayerRel::Opponent,
        }])],
}
