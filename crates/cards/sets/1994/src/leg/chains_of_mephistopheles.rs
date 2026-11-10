//! Chains of Mephistopheles — {1}{B} — Enchantment
//! Oracle: If a player would draw a card except the first one they draw in each of their draw steps, that player discards a card instead. If the player discards a card this way, they draw a card. If the player doesn't discard a card this way, they mill a card.
//! Set: ME1 #63 — Masters Edition | Scryfall ID: f2edb3a6-8506-4885-b332-eca381940ce8 | Oracle ID: eae87919-6322-4bd2-ae9c-b1ce25d686da
// IMPLEMENTED — the whole printed sentence is one draw replacement
// (ReplacementRule::DiscardInsteadOfLaterDraws).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAINS_OF_MEPHISTOPHELES,
    oracle_id = "eae87919-6322-4bd2-ae9c-b1ce25d686da",
    scryfall_id = "f2edb3a6-8506-4885-b332-eca381940ce8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chains of Mephistopheles",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[AbilityDef::Replacement(
        ReplacementRule::DiscardInsteadOfLaterDraws,
    )],
);
