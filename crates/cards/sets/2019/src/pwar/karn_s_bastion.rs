//! Karn's Bastion — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Proliferate. (Choose any number of permanents and/or players, then give each another counter of each kind already there.)
//! Set: EOC #163 — Edge of Eternities Commander | Scryfall ID: 22017ec2-3552-4865-af76-dba042b141f5 | Oracle ID: 9fb8cd81-403a-4988-8f1c-b8eccf8abd9c

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KARN_S_BASTION,
    oracle_id = "9fb8cd81-403a-4988-8f1c-b8eccf8abd9c",
    scryfall_id = "22017ec2-3552-4865-af76-dba042b141f5",
    faces = &[face!(name = "Karn's Bastion", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(cost!("{4}", TapSelf), &[Effect::Proliferate]),
    ],
);
