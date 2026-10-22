//! Wand of Denial — {2} — Artifact
//! Oracle: {T}: Look at the top card of target player's library. If it's a nonland card, you may pay 2 life. If you do, put it into that player's graveyard.
//! Set: 6ED #317 — Classic Sixth Edition | Scryfall ID: f6f37dd8-87d6-432f-a60d-a7a699da3080 | Oracle ID: 9d32da71-60e5-4eb3-97ca-ae76e2f5df57
// PARTIAL: taps to look at the top card of target player's library; optionally paying 2 life if it is a nonland card to put it into that player's graveyard is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAND_OF_DENIAL,
    oracle_id = "9d32da71-60e5-4eb3-97ca-ae76e2f5df57",
    scryfall_id = "f6f37dd8-87d6-432f-a60d-a7a699da3080",
    coverage = Coverage::Partial(
        "taps to look at the top card of target player's library; optionally paying 2 life if it is a nonland card to put it into that player's graveyard is not yet modeled",
    ),
    faces = &[face!(
        name = "Wand of Denial",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::LookAtTopOf {
            player: PlayerRel::Chosen,
            count: 1,
        }],
        target = Some(TargetSpec::AnyPlayer),
    )],
);
