//! Reliquary Tower — (no cost) — Land
//! Oracle: You have no maximum hand size.
//! Oracle: {T}: Add {C}.
//! Set: SOC #398 — Secrets of Strixhaven Commander | Scryfall ID: e2a27742-08c1-4153-af7f-25a7a98f585e | Oracle ID: c23e5b80-08d2-4e24-9908-fe2aa4f30f6f
// IMPLEMENTED — no-max-hand-size modifier + {C} mana.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RELIQUARY_TOWER,
    oracle_id = "c23e5b80-08d2-4e24-9908-fe2aa4f30f6f",
    scryfall_id = "e2a27742-08c1-4153-af7f-25a7a98f585e",
    faces = &[face!(name = "Reliquary Tower", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::Any, Modifier::NoMaxHandSize),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
