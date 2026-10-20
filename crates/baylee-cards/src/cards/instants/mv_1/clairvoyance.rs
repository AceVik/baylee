//! Clairvoyance — {U} — Instant
//! Oracle: Look at target player's hand.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: ICE #63 — Ice Age | Scryfall ID: 46740353-e2ba-4d80-a97d-1368bc67bf30 | Oracle ID: 851333be-d7d5-472a-95cb-0a28da09ce24
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CLAIRVOYANCE,
    oracle_id = "851333be-d7d5-472a-95cb-0a28da09ce24",
    scryfall_id = "46740353-e2ba-4d80-a97d-1368bc67bf30",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Clairvoyance",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::LookAtChosenHand,
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
