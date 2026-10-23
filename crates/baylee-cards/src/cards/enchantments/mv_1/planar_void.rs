//! Planar Void — {B} — Enchantment
//! Oracle: Whenever another card is put into a graveyard from anywhere, exile that card.
//! Set: USG #149 — Urza's Saga | Scryfall ID: b035c718-ac11-4c97-a9bb-0cd88dc71904 | Oracle ID: 994e27e1-0bff-47c9-a1e6-b9e3ae4ffb1e
// PARTIAL: triggering whenever another card is put into a graveyard from anywhere to exile that card is not in Trigger; replaces cards destined for an opponent's graveyard with exile.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANAR_VOID,
    oracle_id = "994e27e1-0bff-47c9-a1e6-b9e3ae4ffb1e",
    scryfall_id = "b035c718-ac11-4c97-a9bb-0cd88dc71904",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "triggering whenever another card is put into a graveyard from anywhere to exile that card is not in Trigger; replaces cards destined for an opponent's graveyard with exile",
    ),
    faces = &[face!(
        name = "Planar Void",
        mana_cost = mana!("{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[AbilityDef::Replacement(
        ReplacementRule::ExileOpponentsGraveyard { counter: None },
    ),],
);
