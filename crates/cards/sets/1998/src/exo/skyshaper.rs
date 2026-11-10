//! Skyshaper — {2} — Artifact
//! Oracle: Sacrifice this artifact: Creatures you control gain flying until end of turn.
//! Set: TPR #231 — Tempest Remastered | Scryfall ID: bfebe9f6-9915-47d2-8565-ca9132ee29ff | Oracle ID: b5642460-883c-4e38-8552-9bea4e50d8e8
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SKYSHAPER,
    oracle_id = "b5642460-883c-4e38-8552-9bea4e50d8e8",
    scryfall_id = "bfebe9f6-9915-47d2-8565-ca9132ee29ff",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Skyshaper",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(SacrificeSelf),
        &[Effect::PumpFilter {
            filter: &Filter::YOUR_CREATURE,
            controlled_by: None,
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::FLYING,
            duration: Duration::UntilEndOfTurn
        }]
    ),],
);
