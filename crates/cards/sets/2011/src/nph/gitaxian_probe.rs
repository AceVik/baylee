//! Gitaxian Probe — {U/P} — Sorcery
//! Oracle: ({U/P} can be paid with either {U} or 2 life.)
//! Oracle: Look at target player's hand.
//! Oracle: Draw a card.
//! Set: NPH #35 — New Phyrexia | Scryfall ID: 995486ce-58bb-4753-a812-0ca73ef1a235 | Oracle ID: 1d67f5ff-1fce-45e5-b6a1-416c569351e2

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GITAXIAN_PROBE,
    oracle_id = "1d67f5ff-1fce-45e5-b6a1-416c569351e2",
    scryfall_id = "995486ce-58bb-4753-a812-0ca73ef1a235",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Gitaxian Probe",
        mana_cost = mana!("{U/P}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::LookAtChosenHand, Effect::draw(1)],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
