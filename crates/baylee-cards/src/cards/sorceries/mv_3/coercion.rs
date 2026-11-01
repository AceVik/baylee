//! Coercion — {2}{B} — Sorcery
//! Oracle: Target opponent reveals their hand. You choose a card from it. That player discards that card.
//! Set: TPR #86 — Tempest Remastered | Scryfall ID: 8c0459ca-1646-46a0-9fc2-709b321bb41f | Oracle ID: 68413337-ddb9-46f8-8c8e-f3d2d672c652
// IMPLEMENTED — target opponent reveals their hand and discards a card you choose from it.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COERCION,
    oracle_id = "68413337-ddb9-46f8-8c8e-f3d2d672c652",
    scryfall_id = "8c0459ca-1646-46a0-9fc2-709b321bb41f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Coercion",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::RevealHandDiscard {
            filter: &Filter::Any,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
