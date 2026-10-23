//! Defense Grid — {2} — Artifact
//! Oracle: Each spell costs {3} more to cast except during its controller's turn.
//! Set: 9ED #293 — Ninth Edition | Scryfall ID: f939dac5-ad71-4163-adde-4f435412477c | Oracle ID: 22ba474e-f6e6-450d-a4cf-0739f0b51079
// PARTIAL: taxing an opponent's spell cast during another opponent's turn in multiplayer is not in Condition; spells cast by opponents during your turn and spells you cast during opponents' turns cost {3} more to cast.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEFENSE_GRID,
    oracle_id = "22ba474e-f6e6-450d-a4cf-0739f0b51079",
    scryfall_id = "f939dac5-ad71-4163-adde-4f435412477c",
    coverage = Coverage::Partial(
        "taxing an opponent's spell cast during another opponent's turn in multiplayer is not in Condition; spells cast by opponents during your turn and spells you cast during opponents' turns cost {3} more to cast",
    ),
    faces = &[face!(
        name = "Defense Grid",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(
            Filter::ControlledByOpponent,
            Modifier::SpellsCostMore(3),
            condition = Some(Condition::YourTurn),
        ),
        static_ability!(
            Filter::ControlledByYou,
            Modifier::SpellsCostMore(3),
            condition = Some(Condition::OpponentsTurn),
        ),
    ],
);
