//! Farsight Mask — {5} — Artifact
//! Oracle: Whenever a source an opponent controls deals damage to you, if this artifact is untapped, you may draw a card.
//! Set: PCA #109 — Planechase Anthology | Scryfall ID: 48bde648-810a-4859-bbdd-265e932e1f7f | Oracle ID: c82091b2-f2f1-4a85-80d6-91ab218f0a94
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FARSIGHT_MASK,
    oracle_id = "c82091b2-f2f1-4a85-80d6-91ab218f0a94",
    scryfall_id = "48bde648-810a-4859-bbdd-265e932e1f7f",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Farsight Mask",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::ControlledByOpponent,
            player: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }],
        }],
        condition = Some(Condition::SourceMatches(&Filter::Untapped)),
    )],
);
