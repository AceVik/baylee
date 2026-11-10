//! Coral Helm — {3} — Artifact
//! Oracle: {3}, Discard a card at random: Target creature gets +2/+2 until end of turn.
//! Set: ME4 #194 — Masters Edition IV | Scryfall ID: 4ac5a963-d3f3-4700-8c10-6c6974b20c24 | Oracle ID: aa2970c8-f2ea-4e06-8b8f-ec89af0012a0
// IMPLEMENTED — seeded random discard is an atomic activation cost.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CORAL_HELM,
    oracle_id = "aa2970c8-f2ea-4e06-8b8f-ec89af0012a0",
    scryfall_id = "4ac5a963-d3f3-4700-8c10-6c6974b20c24",
    faces = &[face!(
        name = "Coral Helm",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{3}", DiscardRandom),
        &[Effect::PumpTarget {
            power: Amount::Fixed(2),
            toughness: Amount::Fixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    )],
);
