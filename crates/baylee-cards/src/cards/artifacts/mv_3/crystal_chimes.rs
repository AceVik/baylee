//! Crystal Chimes — {3} — Artifact
//! Oracle: {3}, {T}, Sacrifice this artifact: Return all enchantment cards from your graveyard to your hand.
//! Set: USG #292 — Urza's Saga | Scryfall ID: d5d03d54-7aa4-4586-bb4b-8a5d269fe289 | Oracle ID: fbff7452-1da4-4cd0-8247-a625e4b4da2b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRYSTAL_CHIMES,
    oracle_id = "fbff7452-1da4-4cd0-8247-a625e4b4da2b",
    scryfall_id = "d5d03d54-7aa4-4586-bb4b-8a5d269fe289",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crystal Chimes",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf, SacrificeSelf),
        &[Effect::GraveyardAllToHand {
            filter: &Filter::ENCHANTMENT,
        }],
    )],
);
