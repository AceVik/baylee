//! Watchdog — {3} — Artifact Creature — Dog
//! Oracle: This creature blocks each combat if able.
//! Oracle: As long as this creature is untapped, all creatures attacking you get -1/-0.
//! Set: TMP #314 — Tempest | Scryfall ID: 8c2ffc07-9993-40de-b36e-33c7afd4cfc2 | Oracle ID: 6a951d2d-fc5a-4f41-bd77-3e301b76cf47
// IMPLEMENTED — blocks each combat if able and gives -1/-0 to all creatures attacking you while untapped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WATCHDOG,
    oracle_id = "6a951d2d-fc5a-4f41-bd77-3e301b76cf47",
    scryfall_id = "8c2ffc07-9993-40de-b36e-33c7afd4cfc2",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Watchdog",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::DOG],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::BlocksEachAttackerIfAble),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::Attacking, Filter::AttackingYou]),
            Modifier::ModifyPT(-1, 0),
            condition = Some(Condition::SourceMatches(&Filter::Untapped)),
        ),
    ],
);
