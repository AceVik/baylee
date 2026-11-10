//! Delif's Cube — {1} — Artifact
//! Oracle: {2}, {T}: This turn, when target creature you control attacks and isn't blocked, it assigns no combat damage this turn and you put a cube counter on this artifact.
//! Oracle: {2}, Remove a cube counter from this artifact: Regenerate target creature.
//! Set: FEM #85 — Fallen Empires | Scryfall ID: 14749600-9eca-4122-b04f-30ddda091b74 | Oracle ID: 3106ba15-08d7-474c-a384-6848b713ba8d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DELIF_S_CUBE,
    oracle_id = "3106ba15-08d7-474c-a384-6848b713ba8d",
    scryfall_id = "14749600-9eca-4122-b04f-30ddda091b74",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Delif's Cube",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::WhenTargetAttacksAndIsntBlockedThisTurn {
                effects: &[
                    Effect::ModifyEvent {
                        modifier: Modifier::AssignsNoCombatDamage,
                        duration: Duration::UntilEndOfTurn,
                    },
                    Effect::AddCounterSource {
                        kind: counters::CUBE,
                        amount: Amount::Fixed(1),
                    },
                ],
            }],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
        ),
        activated!(
            cost!(
                "{2}",
                RemoveCounterSelf {
                    kind: counters::CUBE,
                    n: 1,
                }
            ),
            &[Effect::regenerate(TargetSpec::Object(&Filter::CREATURE))],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
