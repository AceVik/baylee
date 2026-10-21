//! Ashnod's Cylix — {2} — Artifact
//! Oracle: {3}, {T}: Target player looks at the top three cards of their library, puts one of them back on top of their library, then exiles the rest.
//! Set: ME2 #203 — Masters Edition II | Scryfall ID: b625b911-8b3c-4a31-a9cf-7403355c5551 | Oracle ID: fdf4e4c3-fe17-4098-9f76-e98f3f00bd3e
// NOT SUPPORTED: having target player look at the top three cards of their library, put one back on top, and exile the rest is not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASHNOD_S_CYLIX,
    oracle_id = "fdf4e4c3-fe17-4098-9f76-e98f3f00bd3e",
    scryfall_id = "b625b911-8b3c-4a31-a9cf-7403355c5551",
    coverage = Coverage::Partial(
        "{3}, {T} looks at the top three cards of target player's library; having that player look at their own top three cards, put one back on top, and exile the rest is not supported",
    ),
    faces = &[face!(
        name = "Ashnod's Cylix",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[Effect::LookAtTopOf {
            player: PlayerRel::Chosen,
            count: 3,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
