//! Vibrating Sphere — {4} — Artifact
//! Oracle: During your turn, creatures you control get +2/+0.
//! Oracle: During turns other than yours, creatures you control get -0/-2.
//! Set: ME4 #238 — Masters Edition IV | Scryfall ID: 7f0da2b7-ed3b-47fd-962e-ca5dc745cfc7 | Oracle ID: 8a8fe94f-81aa-4725-ace6-ed56a1e1ed91

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VIBRATING_SPHERE,
    oracle_id = "8a8fe94f-81aa-4725-ace6-ed56a1e1ed91",
    scryfall_id = "7f0da2b7-ed3b-47fd-962e-ca5dc745cfc7",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vibrating Sphere",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::ModifyPT(2, 0),
            condition = Some(Condition::YourTurn)
        ),
        static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::ModifyPT(0, -2),
            condition = Some(Condition::Not(&Condition::YourTurn))
        ),
    ],
);
