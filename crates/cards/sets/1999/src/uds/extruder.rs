//! Extruder — {4} — Artifact Creature — Juggernaut
//! Oracle: Echo {4} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: Sacrifice an artifact: Put a +1/+1 counter on target creature.
//! Set: MH2 #296 — Modern Horizons 2 | Scryfall ID: e94e53f0-cfca-4b40-9867-94ea771463cd | Oracle ID: 93cc6425-a13b-4b32-bb8f-fdc9bfa0ef9b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EXTRUDER,
    oracle_id = "93cc6425-a13b-4b32-bb8f-fdc9bfa0ef9b",
    scryfall_id = "e94e53f0-cfca-4b40-9867-94ea771463cd",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Extruder",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::JUGGERNAUT],
        power = Some(4),
        toughness = Some(3),
    ),],
    abilities = &[
        AbilityDef::Echo { cost: mana!("{4}") },
        activated!(
            cost!(Sacrifice(&Filter::YOUR_ARTIFACT)),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ],
);
