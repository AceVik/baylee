//! The Meathook Massacre — {X}{B}{B} — Legendary Enchantment
//! Oracle: When The Meathook Massacre enters, each creature gets -X/-X until end of turn.
//! Oracle: Whenever a creature you control dies, each opponent loses 1 life.
//! Oracle: Whenever a creature an opponent controls dies, you gain 1 life.
//! Set: INR #122 — Innistrad Remastered | Scryfall ID: 70d0540f-93c6-4af5-ab2d-65e6c03001c7 | Oracle ID: 127de52b-df75-4342-95a0-20d84c5bf916
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_MEATHOOK_MASSACRE,
    oracle_id = "127de52b-df75-4342-95a0-20d84c5bf916",
    scryfall_id = "70d0540f-93c6-4af5-ab2d-65e6c03001c7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "The Meathook Massacre",
        mana_cost = mana!("{X}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
