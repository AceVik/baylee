//! Headstone — {1}{B} — Instant
//! Oracle: Exile target card from a graveyard.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: HML #52 — Homelands | Scryfall ID: 2fdae7fa-1076-4ff3-b771-fc3f5d9ba89f | Oracle ID: cbf767d4-9be4-417a-bbf7-0c72ede727b4
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEADSTONE,
    oracle_id = "cbf767d4-9be4-417a-bbf7-0c72ede727b4",
    scryfall_id = "2fdae7fa-1076-4ff3-b771-fc3f5d9ba89f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Headstone",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::Exile {
                target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer),
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::Any,
            PlayerRel::EachPlayer,
        )))
    )],
);
