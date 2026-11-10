//! Ghost Town — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {0}: Return this land to its owner's hand. Activate only if it's not your turn.
//! Set: TMP #318 — Tempest | Scryfall ID: 4218cdda-3a62-43fb-aaf7-7ac836392796 | Oracle ID: f2c861d3-b302-4e84-b647-099551007269

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GHOST_TOWN,
    oracle_id = "f2c861d3-b302-4e84-b647-099551007269",
    scryfall_id = "4218cdda-3a62-43fb-aaf7-7ac836392796",
    faces = &[face!(name = "Ghost Town", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{0}"),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject
            }],
            condition = Some(Condition::Not(&Condition::YourTurn))
        ),
    ],
);
