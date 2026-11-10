//! Hurkyl's Recall — {1}{U} — Instant
//! Oracle: Return all artifacts target player owns to their hand.
//! Set: MM2 #48 — Modern Masters 2015 | Scryfall ID: 73edeaaa-6a87-4cf1-b013-bab9a7bb94d9 | Oracle ID: ed1e5d24-c8a8-48fe-a88f-1003ad432477
// IMPLEMENTED — a target player; every artifact they own, whoever controls it, returns to their hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HURKYL_S_RECALL,
    oracle_id = "ed1e5d24-c8a8-48fe-a88f-1003ad432477",
    scryfall_id = "73edeaaa-6a87-4cf1-b013-bab9a7bb94d9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Hurkyl's Recall",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::ReturnAllOwnedToHand {
            filter: &Filter::ARTIFACT,
            owner: PlayerRel::Chosen,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
