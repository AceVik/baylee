//! Sequestered Stash — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}, Sacrifice this land: Mill five cards. Then you may put an artifact card from your graveyard on top of your library.
//! Set: KLD #248 — Kaladesh | Scryfall ID: 86a17084-bb96-4e81-bff0-005bd44a1fbd | Oracle ID: b6fe779f-b20d-49cc-96dd-54f1ffb312e1

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEQUESTERED_STASH,
    oracle_id = "b6fe779f-b20d-49cc-96dd-54f1ffb312e1",
    scryfall_id = "86a17084-bb96-4e81-bff0-005bd44a1fbd",
    faces = &[face!(name = "Sequestered Stash", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}", TapSelf, SacrificeSelf),
            &[
                Effect::Mill {
                    amount: Amount::Fixed(5),
                    target: PlayerRel::You
                },
                Effect::ChooseFromYourGraveyard {
                    filter: &Filter::ARTIFACT,
                    destination: SearchDest::TopOfLibrary,
                    tapped: false,
                    optional: true
                }
            ]
        ),
    ],
);
