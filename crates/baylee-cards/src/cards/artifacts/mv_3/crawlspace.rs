//! Crawlspace — {3} — Artifact
//! Oracle: No more than two creatures can attack you each combat.
//! Set: DMR #217 — Dominaria Remastered | Scryfall ID: 5b8e092a-610d-4cea-bb92-65e01954b5c4 | Oracle ID: 2296370c-fe34-4df6-92a5-260f1634bede
// PARTIAL: limiting attackers only when attacking you is not separated from global MaxAttackersPerCombat; limits each combat to at most two attacking creatures.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRAWLSPACE,
    oracle_id = "2296370c-fe34-4df6-92a5-260f1634bede",
    scryfall_id = "5b8e092a-610d-4cea-bb92-65e01954b5c4",
    coverage = Coverage::Partial(
        "limiting attackers only when attacking you is not separated from global MaxAttackersPerCombat; limits each combat to at most two attacking creatures",
    ),
    faces = &[face!(
        name = "Crawlspace",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::CREATURE,
        Modifier::MaxAttackersPerCombat(2),
    )],
);
