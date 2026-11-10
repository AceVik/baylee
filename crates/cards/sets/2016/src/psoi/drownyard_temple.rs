//! Drownyard Temple — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}: Return this card from your graveyard to the battlefield tapped.
//! Set: TDC #359 — Tarkir: Dragonstorm Commander | Scryfall ID: dbc8512a-9f6c-40d4-8049-14505e260746 | Oracle ID: c30f9be4-c274-4ad0-b5d7-7d3421aa4277

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DROWNYARD_TEMPLE,
    oracle_id = "c30f9be4-c274-4ad0-b5d7-7d3421aa4277",
    scryfall_id = "dbc8512a-9f6c-40d4-8049-14505e260746",
    faces = &[face!(name = "Drownyard Temple", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{3}"),
            &[Effect::ReturnToBattlefieldTapped {
                target: TargetSpec::ThisObject
            }],
            zone = ActivationZone::Graveyard,
        ),
    ],
);
