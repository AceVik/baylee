//! Soulstone Sanctuary — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}: This land becomes a 3/3 creature with vigilance and all creature types. It's still a land.
//! Set: FDN #133 — Foundations | Scryfall ID: 642553a7-6d0f-483d-a873-3a703786db42 | Oracle ID: 3c0f99b8-0222-4fac-932a-eb5d77826564
// IMPLEMENTED — {T}: Add {C}; {4} animates this land indefinitely as a
// 3/3 vigilance creature with every creature type, still a land.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOULSTONE_SANCTUARY,
    oracle_id = "3c0f99b8-0222-4fac-932a-eb5d77826564",
    scryfall_id = "642553a7-6d0f-483d-a873-3a703786db42",
    faces = &[face!(name = "Soulstone Sanctuary", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::Indefinitely,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AllCreatureTypes,
                    Duration::Indefinitely,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::VIGILANCE),
                    Duration::Indefinitely,
                ),
                Effect::continuous(&Filter::This, Modifier::SetPT(3, 3), Duration::Indefinitely,),
            ],
        ),
    ],
);
