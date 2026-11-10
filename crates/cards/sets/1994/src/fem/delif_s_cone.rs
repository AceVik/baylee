//! Delif's Cone — {0} — Artifact
//! Oracle: {T}, Sacrifice this artifact: This turn, when target creature you control attacks and isn't blocked, you may gain life equal to its power. If you do, it assigns no combat damage this turn.
//! Set: FEM #84 — Fallen Empires | Scryfall ID: 262b8788-c5a0-4c8e-9d58-b769b1b0a2ff | Oracle ID: 8af33199-d3ad-44fe-8ee0-abbe36a2278b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DELIF_S_CONE,
    oracle_id = "8af33199-d3ad-44fe-8ee0-abbe36a2278b",
    scryfall_id = "262b8788-c5a0-4c8e-9d58-b769b1b0a2ff",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Delif's Cone",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[Effect::WhenTargetAttacksAndIsntBlockedThisTurn {
            effects: &[Effect::MayDo {
                effects: &[
                    Effect::GainLife {
                        amount: Amount::EventPower,
                    },
                    Effect::ModifyEvent {
                        modifier: Modifier::AssignsNoCombatDamage,
                        duration: Duration::UntilEndOfTurn,
                    },
                ],
            }],
        }],
        target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
    ),],
);
