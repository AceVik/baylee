//! Sanctum of Ugin — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: Whenever you cast a colorless spell with mana value 7 or greater, you may sacrifice this land. If you do, search your library for a colorless creature card, reveal it, put it into your hand, then shuffle.
//! Set: BFZ #242 — Battle for Zendikar | Scryfall ID: 86798d03-9f2d-46bd-a660-13c8dd5535ce | Oracle ID: 72cb5dcd-9b24-435c-921a-3766108374c4
// IMPLEMENTED — {T}: Add {C}, and the cast trigger as one Effect::MayDo over
// Effect::SacrificeSelf and a filtered Effect::SearchLibrary. The two halves
// of the search the card prints but the DSL derives — "reveal it" (a hidden
// destination behind a filter narrower than "a card") and "then shuffle" —
// come from the search itself.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SANCTUM_OF_UGIN,
    oracle_id = "72cb5dcd-9b24-435c-921a-3766108374c4",
    scryfall_id = "86798d03-9f2d-46bd-a660-13c8dd5535ce",
    faces = &[face!(name = "Sanctum of Ugin", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        triggered!(
            Trigger::SpellCast(&Filter::And(&[
                Filter::ControlledByYou,
                Filter::IsColorless,
                Filter::CmcAtLeast(7),
            ])),
            &[Effect::MayDo {
                effects: &[
                    Effect::SacrificeSelf,
                    Effect::SearchLibrary {
                        filter: &f!(colorless CREATURE),
                        finds: &[Find::HAND],
                        optional: false,
                    },
                ],
            }]
        ),
    ],
);
