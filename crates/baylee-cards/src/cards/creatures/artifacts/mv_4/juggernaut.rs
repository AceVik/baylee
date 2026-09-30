//! Juggernaut — {4} — Artifact Creature — Juggernaut
//! Oracle: This creature attacks each combat if able.
//! Oracle: This creature can't be blocked by Walls.
//! Set: FDN #255 — Foundations | Scryfall ID: f4468fff-cd6f-428c-b7a0-ff89f5bbea2e | Oracle ID: 4ac9116f-36bc-4d71-b696-d6ee064e1d58
// PARTIAL — attacking each combat if able is not in the engine; Walls can't
// block it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static WALLS: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::WALL),
]);

card!(
    index = index::JUGGERNAUT,
    oracle_id = "4ac9116f-36bc-4d71-b696-d6ee064e1d58",
    scryfall_id = "f4468fff-cd6f-428c-b7a0-ff89f5bbea2e",
    coverage = Coverage::Partial(
        "attacking each combat if able is not in the engine; Walls can't block it"
    ),
    faces = &[face!(
        name = "Juggernaut",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::JUGGERNAUT],
        power = Some(5),
        toughness = Some(3),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::CantBeBlockedBy(&WALLS)),
        // NOT SUPPORTED: This creature attacks each combat if able.
    ],
);
