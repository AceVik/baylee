//! Mercadian Atlas — {5} — Artifact — Book
//! Oracle: At the beginning of your end step, if you didn't play a land this turn, you may draw a card.
//! Set: MMQ #305 — Mercadian Masques | Scryfall ID: 00ad3531-399c-4897-b0ee-ad2a26445a17 | Oracle ID: 5bae9ffb-99b3-4feb-8aed-265d874fd68b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MERCADIAN_ATLAS,
    oracle_id = "5bae9ffb-99b3-4feb-8aed-265d874fd68b",
    scryfall_id = "00ad3531-399c-4897-b0ee-ad2a26445a17",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mercadian Atlas",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::BOOK],
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::draw(1)],
        }],
        condition = Some(Condition::Not(&Condition::LandsPlayedThisTurnAtLeast(1))),
    ),],
);
