//! Sanctum of Eternity — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Return target commander you own from the battlefield to your hand. Activate only during your turn.
//! Set: C19 #59 — Commander 2019 | Scryfall ID: a680cc6d-9c31-4f5d-808f-dace92bf4346 | Oracle ID: c7d9ff27-f1fc-42e4-a47b-d2e6d68e4035

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SANCTUM_OF_ETERNITY,
    oracle_id = "c7d9ff27-f1fc-42e4-a47b-d2e6d68e4035",
    scryfall_id = "a680cc6d-9c31-4f5d-808f-dace92bf4346",
    faces = &[face!(name = "Sanctum of Eternity", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::bounce(TargetSpec::Object(&Filter::And(&[
                Filter::IsCommander,
                Filter::OwnedByYou
            ])))],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::IsCommander,
                Filter::OwnedByYou
            ]))),
            condition = Some(Condition::YourTurn)
        )
    ],
);
