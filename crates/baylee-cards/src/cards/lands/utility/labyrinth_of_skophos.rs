//! Labyrinth of Skophos — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Remove target attacking or blocking creature from combat.
//! Set: MKC #272 — Murders at Karlov Manor Commander | Scryfall ID: 388169df-4cce-452d-9215-e3f814ff4fdf | Oracle ID: 9ec5a487-d8ed-459a-8f58-56f6e9a2dfe8

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LABYRINTH_OF_SKOPHOS,
    oracle_id = "9ec5a487-d8ed-459a-8f58-56f6e9a2dfe8",
    scryfall_id = "388169df-4cce-452d-9215-e3f814ff4fdf",
    faces = &[face!(name = "Labyrinth of Skophos", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::RemoveTargetFromCombat { unblock: false }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::CREATURE,
                Filter::Or(&[Filter::Attacking, Filter::Blocking]),
            ]))),
        ),
    ],
);
