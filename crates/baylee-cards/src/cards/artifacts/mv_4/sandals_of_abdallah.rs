//! Sandals of Abdallah — {4} — Artifact
//! Oracle: {2}, {T}: Target creature gains islandwalk until end of turn. When that creature dies this turn, destroy this artifact. (A creature with islandwalk can't be blocked as long as defending player controls an Island.)
//! Set: ARN #69 — Arabian Nights | Scryfall ID: 8f99a520-b8a9-40b0-9854-48aac297c5ee | Oracle ID: ff77074f-48ef-4c01-8ede-4e9be3e483f4
// PARTIAL — the islandwalk pump is written; the delayed trigger that destroys
// this artifact when the pumped creature dies is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SANDALS_OF_ABDALLAH,
    oracle_id = "ff77074f-48ef-4c01-8ede-4e9be3e483f4",
    scryfall_id = "8f99a520-b8a9-40b0-9854-48aac297c5ee",
    faces = &[face!(
        name = "Sandals of Abdallah",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "the delayed trigger that destroys this artifact when the pumped \
         creature dies this turn is not expressible"
    ),
    // NOT SUPPORTED: "When that creature dies this turn, destroy this
    // artifact." — the engine does keep a watch for exactly this shape
    // (`DelayedWhen::DiesOrIsExiled`), but only `Effect::Earthbend` writes
    // one, for the land it animated, and no DSL effect installs such a watch
    // on a chosen target. `Effect::AtNextEndStep` is the nearest delayed
    // trigger and fires at the end step whether or not the creature died, so
    // it would destroy the artifact on a schedule the card does not print.
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::ISLANDWALK,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    )],
);
