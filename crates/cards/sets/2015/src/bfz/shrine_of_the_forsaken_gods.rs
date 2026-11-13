//! Shrine of the Forsaken Gods — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {C}{C}. Spend this mana only to cast colorless spells. Activate only if you control seven or more lands.
//! Set: MKC #292 — Murders at Karlov Manor Commander | Scryfall ID: 4472d5fb-ca8f-43d0-a95d-5bd8bdb6a02d | Oracle ID: 8ea46945-d5ab-4209-b473-4769e7b8b962
// IMPLEMENTED — {C} unconditionally; {C}{C} restricted to colorless spells
// and offered only with seven or more lands (Condition::ControlCount).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHRINE_OF_THE_FORSAKEN_GODS,
    oracle_id = "8ea46945-d5ab-4209-b473-4769e7b8b962",
    scryfall_id = "4472d5fb-ca8f-43d0-a95d-5bd8bdb6a02d",
    faces = &[face!(
        name = "Shrine of the Forsaken Gods",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[Effect::mana(ManaColor::Colorless, 2)
                .restricted(&Filter::IsColorless, SpendRider::None)],
            condition = Some(Condition::ControlCount(&Filter::LAND, 7))
        ),
    ],
);
