//! No Rest for the Wicked — {1}{B} — Enchantment
//! Oracle: Sacrifice this enchantment: Return to your hand all creature cards in your graveyard that were put there from the battlefield this turn.
//! Set: USG #142 — Urza's Saga | Scryfall ID: d9f9b2b0-65aa-42a1-b9f8-563b194ca5b1 | Oracle ID: d03fd358-ff3f-45ab-9601-07dbb535cb51
// PARTIAL

use baylee_cards_dsl::prelude::*;

const CREATURE_TO_GY_THIS_TURN: Filter =
    Filter::And(&[Filter::CREATURE, Filter::PutIntoGraveyardThisTurn]);

card!(
    index = index::NO_REST_FOR_THE_WICKED,
    oracle_id = "d03fd358-ff3f-45ab-9601-07dbb535cb51",
    scryfall_id = "d9f9b2b0-65aa-42a1-b9f8-563b194ca5b1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Filter::PutIntoGraveyardThisTurn matches creature cards put into your graveyard from anywhere this turn rather than only from the battlefield; sacrifices itself to return all creature cards put into your graveyard this turn to your hand",
    ),
    faces = &[face!(
        name = "No Rest for the Wicked",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(SacrificeSelf),
        &[Effect::GraveyardAllToHand {
            filter: &CREATURE_TO_GY_THIS_TURN,
        }],
    )],
);
