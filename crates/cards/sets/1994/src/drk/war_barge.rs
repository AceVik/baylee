//! War Barge — {4} — Artifact
//! Oracle: {3}: Target creature gains islandwalk until end of turn. When this artifact leaves the battlefield this turn, destroy that creature. A creature destroyed this way can't be regenerated. (A creature with islandwalk can't be blocked as long as defending player controls an Island.)
//! Set: TSB #116 — Time Spiral Timeshifted | Scryfall ID: 3c26771c-a1ec-4055-8df4-d5b49bfe5b1e | Oracle ID: fd57331f-1cfa-4f88-a925-fc171738c3c0

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAR_BARGE,
    oracle_id = "fd57331f-1cfa-4f88-a925-fc171738c3c0",
    scryfall_id = "3c26771c-a1ec-4055-8df4-d5b49bfe5b1e",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "War Barge",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}"),
        &[
            Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::ISLANDWALK),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::WhenSourceLeavesBattlefieldThisTurn {
                effects: &[Effect::Destroy {
                    target: TargetSpec::EventObject,
                    no_regen: true,
                }],
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    )],
);
