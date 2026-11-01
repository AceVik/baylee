//! Well of Knowledge — {3} — Artifact
//! Oracle: {2}: Draw a card. Any player may activate this ability but only during their draw step.
//! Set: WTH #162 — Weatherlight | Scryfall ID: 5184b967-f474-4c9b-9a20-65ddb0d6e4f8 | Oracle ID: 99e2b36b-1fab-4b09-b371-d97a6854dfbc
// IMPLEMENTED — {2}: Draw a card, activatable by any player only during their draw step.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WELL_OF_KNOWLEDGE,
    oracle_id = "99e2b36b-1fab-4b09-b371-d97a6854dfbc",
    scryfall_id = "5184b967-f474-4c9b-9a20-65ddb0d6e4f8",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Well of Knowledge",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}"),
        &[Effect::draw(1)],
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Draw),
        ])),
        zone = ActivationZone::BattlefieldAnyPlayer
    ),],
);
