//! Watermarket — (no cost) — Land
//! Oracle: {T}: Add {C}{C}. Spend this mana only to cast spells with watermarks.
//! Set: UST #166 — Unstable | Scryfall ID: bf98bb09-c479-4dba-b542-ac5475fac697 | Oracle ID: 84d89a3d-4b28-4e19-8298-737ec6a06238

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WATERMARKET,
    oracle_id = "84d89a3d-4b28-4e19-8298-737ec6a06238",
    scryfall_id = "bf98bb09-c479-4dba-b542-ac5475fac697",
    faces = &[face!(name = "Watermarket", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities =
        &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 2)
            .restricted(&Filter::HasWatermark, SpendRider::None)])],
);
