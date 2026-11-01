//! Ticking Gnomes — {3} — Artifact Creature — Gnome
//! Oracle: Echo {3} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: Sacrifice this creature: It deals 1 damage to any target.
//! Set: EMA #233 — Eternal Masters | Scryfall ID: db6358cf-fcb9-42af-9755-dd1f39bfc8ff | Oracle ID: 963de959-848d-48df-90fa-88a6d988b9fe
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TICKING_GNOMES,
    oracle_id = "963de959-848d-48df-90fa-88a6d988b9fe",
    scryfall_id = "db6358cf-fcb9-42af-9755-dd1f39bfc8ff",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ticking Gnomes",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GNOME],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[
        AbilityDef::Echo { cost: mana!("{3}") },
        activated!(
            cost!(SacrificeSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
    ],
);
