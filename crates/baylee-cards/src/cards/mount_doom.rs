//! Mount Doom — (no cost) — Legendary Land
//! Oracle: {T}, Pay 1 life: Add {B} or {R}.
//! Oracle: {1}{B}{R}, {T}: Mount Doom deals 1 damage to each opponent.
//! Oracle: {5}{B}{R}, {T}, Sacrifice Mount Doom and a legendary artifact: Choose up to two creatures, then destroy the rest. Activate only as a sorcery.
//! Set: LTR #258 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: b5bc71a1-2344-4bc6-aa60-658cec19d0d6 | Oracle ID: 995c8dac-fd27-468a-abd4-02372cf0c850
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 785,
    oracle_id: "995c8dac-fd27-468a-abd4-02372cf0c850",
    scryfall_id: "b5bc71a1-2344-4bc6-aa60-658cec19d0d6",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces: &[
    face! {
        name: "Mount Doom",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
