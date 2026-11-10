//! Goblin Charbelcher — {4} — Artifact
//! Oracle: {3}, {T}: Reveal cards from the top of your library until you reveal a land card. This artifact deals damage equal to the number of nonland cards revealed this way to any target. If the revealed land card was a Mountain, this artifact deals double that damage instead. Put the revealed cards on the bottom of your library in any order.
//! Set: DDT #57 — Duel Decks: Merfolk vs. Goblins | Scryfall ID: 240344ff-404d-4894-a6e9-4401cd68cf50 | Oracle ID: c9b1557b-70b1-44f2-9fc2-7aa0b035b187
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GOBLIN_CHARBELCHER,
    oracle_id = "c9b1557b-70b1-44f2-9fc2-7aa0b035b187",
    scryfall_id = "240344ff-404d-4894-a6e9-4401cd68cf50",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Charbelcher",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[Effect::RevealUntilLandDealDamage {
            target: TargetSpec::AnyTarget,
        }],
        target = Some(TargetSpec::AnyTarget),
    )],
);
