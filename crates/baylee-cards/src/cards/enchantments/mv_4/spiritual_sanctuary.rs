//! Spiritual Sanctuary — {2}{W}{W} — Enchantment
//! Oracle: At the beginning of each player's upkeep, if that player controls a Plains, they gain 1 life.
//! Set: LEG #38 — Legends | Scryfall ID: 654dd1e0-a91d-44ee-af20-c025bf360c3f | Oracle ID: 91edef61-2487-405c-a3ec-67a814dfeff2
// IMPLEMENTED — at each player's upkeep, that player gains 1 life while they control a Plains.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPIRITUAL_SANCTUARY,
    oracle_id = "91edef61-2487-405c-a3ec-67a814dfeff2",
    scryfall_id = "654dd1e0-a91d-44ee-af20-c025bf360c3f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spiritual Sanctuary",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer
        },
        &[Effect::GainLifeFor {
            amount: Amount::Fixed(1),
            who: PlayerRel::ActivePlayer
        }],
        condition = Some(Condition::BattlefieldCount(
            &Filter::And(&[
                Filter::HasSubtype(subtypes::land::PLAINS),
                Filter::ControlledByActivePlayer
            ]),
            1
        ))
    ),],
);
