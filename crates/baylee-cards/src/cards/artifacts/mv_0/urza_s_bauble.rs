//! Urza's Bauble — {0} — Artifact
//! Oracle: {T}, Sacrifice this artifact: Look at a card at random in target player's hand. You draw a card at the beginning of the next turn's upkeep.
//! Set: ME1 #170 — Masters Edition | Scryfall ID: 29175e6b-a2ab-4373-8093-cb372aa8c7d0 | Oracle ID: 17dbbca3-ac1c-4d4e-9618-3e66ac3ccd24
// NOT SUPPORTED: looking privately at a random card in target player's hand is not supported; reveals a random card from target player's hand to all players instead, then draws a card at the beginning of the next turn's upkeep.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URZA_S_BAUBLE,
    oracle_id = "17dbbca3-ac1c-4d4e-9618-3e66ac3ccd24",
    scryfall_id = "29175e6b-a2ab-4373-8093-cb372aa8c7d0",
    coverage = Coverage::Partial(
        "looking privately at a random card in target player's hand is not supported; reveals a random card from target player's hand to all players instead, then draws a card at the beginning of the next turn's upkeep",
    ),
    faces = &[face!(
        name = "Urza's Bauble",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[
            Effect::RevealRandomFromHand {
                who: PlayerRel::Chosen,
                count: Amount::Fixed(1),
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        target = Some(TargetSpec::AnyPlayer)
    )],
);
