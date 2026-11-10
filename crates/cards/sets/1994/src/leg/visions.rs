//! Visions — {W} — Sorcery
//! Oracle: Look at the top five cards of target player's library. You may then have that player shuffle that library.
//! Set: 4ED #54 — Fourth Edition | Scryfall ID: b0fed0e6-0e56-4987-b0dd-b294156c0233 | Oracle ID: 448fa4e3-7269-446b-8089-675bc1bff5f9

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VISIONS,
    oracle_id = "448fa4e3-7269-446b-8089-675bc1bff5f9",
    scryfall_id = "b0fed0e6-0e56-4987-b0dd-b294156c0233",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Visions",
        mana_cost = mana!("{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::LookAtTopOf {
                player: PlayerRel::Chosen,
                count: 5
            },
            Effect::MayDo {
                effects: &[Effect::ShuffleLibrary {
                    who: PlayerRel::Chosen
                }]
            }
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
