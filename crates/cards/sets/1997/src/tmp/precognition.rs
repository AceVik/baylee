//! Precognition — {4}{U} — Enchantment
//! Oracle: At the beginning of your upkeep, you may look at the top card of target opponent's library. If you do, you may put that card on the bottom of that player's library.
//! Set: TMP #79 — Tempest | Scryfall ID: 76a0e317-5a76-4eac-a903-b0e3f0a45873 | Oracle ID: 6d1a9d06-028f-487a-b490-27100c128ea3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRECOGNITION,
    oracle_id = "6d1a9d06-028f-487a-b490-27100c128ea3",
    scryfall_id = "76a0e317-5a76-4eac-a903-b0e3f0a45873",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Precognition",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::ScryFor {
                player: PlayerRel::Chosen,
                amount: Amount::Fixed(1),
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
