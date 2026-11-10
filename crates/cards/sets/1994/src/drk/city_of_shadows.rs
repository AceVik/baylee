//! City of Shadows — (no cost) — Land
//! Oracle: {T}, Exile a creature you control: Put a storage counter on this land.
//! Oracle: {T}: Add {C} for each storage counter on this land.
//! Set: ME3 #206 — Masters Edition III | Scryfall ID: 0629322b-e9e8-4cac-abe5-fff083bcc240 | Oracle ID: 043192d8-6077-46c3-b43f-b7caf6762869

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CITY_OF_SHADOWS,
    oracle_id = "043192d8-6077-46c3-b43f-b7caf6762869",
    scryfall_id = "0629322b-e9e8-4cac-abe5-fff083bcc240",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "City of Shadows", types = TypeSet::LAND,),],
    abilities = &[
        activated!(
            cost!(TapSelf, ExilePermanent(&Filter::YOUR_CREATURE)),
            &[Effect::AddCounter {
                kind: counters::STORAGE,
                amount: Amount::Fixed(1),
            }]
        ),
        mana_ability!(&[Effect::mana_dynamic(
            ManaColor::Colorless,
            Amount::CountersOnSource(counters::STORAGE),
        )]),
    ],
);
