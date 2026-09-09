//! Sidequest: Catch a Fish // Cooking Campsite — {2}{W} — Enchantment // Land
//! Oracle: At the beginning of your upkeep, look at the top card of your library. If it's an artifact or creature card, you may reveal it and put it into your hand. If you put a card into your hand this way, create a Food token and transform this enchantment.
//! Oracle: {T}: Add {W}.
//! Oracle: {3}, {T}, Sacrifice an artifact: Put a +1/+1 counter on each creature you control. Activate only as a sorcery.
//! Set: FIN #31 — Final Fantasy | Scryfall ID: bdb5452e-d97f-409b-91d0-2664f39b09b8 | Oracle ID: bd7c328e-0380-46f8-bb85-7bf4e201b7ac
//! Face: Sidequest: Catch a Fish — {2}{W} — Enchantment
//! Face: Cooking Campsite —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1030,
    oracle_id: "bd7c328e-0380-46f8-bb85-7bf4e201b7ac",
    scryfall_id: "bdb5452e-d97f-409b-91d0-2664f39b09b8",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Sidequest: Catch a Fish",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::ENCHANTMENT,
    },
    face! {
        name: "Cooking Campsite",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
