//! Ostracize — {B} — Sorcery
//! Oracle: Target opponent reveals their hand. You choose a creature card from it. That player discards that card.
//! Set: 7ED #153 — Seventh Edition | Scryfall ID: abdaffcc-59f6-4489-88bf-1061ad6b0512 | Oracle ID: 43975c07-103c-4664-b7e6-55bb29d85183
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OSTRACIZE,
    oracle_id = "43975c07-103c-4664-b7e6-55bb29d85183",
    scryfall_id = "abdaffcc-59f6-4489-88bf-1061ad6b0512",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ostracize",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::RevealHandDiscard {
            filter: &Filter::CREATURE,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
