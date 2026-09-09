//! Sensei's Divining Top — {1} — Artifact
//! Oracle: {1}: Look at the top three cards of your library, then put them back in any order.
//! Oracle: {T}: Draw a card, then put Sensei's Divining Top on top of its owner's library.
//! Set: EMA #232 — Eternal Masters | Scryfall ID: e5142b7a-e580-4737-a4aa-2590f6610ceb | Oracle ID: 13575cf9-65c1-4861-b21e-eb2155e07766
// IMPLEMENTED — top-3 reorder + draw-and-replace.

use baylee_cards_dsl::prelude::*;

card! {
    index: 142,
    oracle_id: "13575cf9-65c1-4861-b21e-eb2155e07766",
    scryfall_id: "e5142b7a-e580-4737-a4aa-2590f6610ceb",
    faces: &[face! {
        name: "Sensei's Divining Top",
        mana_cost: baylee_core::mana!("{1}"),
        types: TypeSet::ARTIFACT,
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        activated!(Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[],
            }, &[Effect::ReorderTopLibrary { count: 3 }]),
        activated!(Cost::TAP, &[
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
                Effect::PutSourceOnTopOfLibrary,
            ]),
    ],
}
