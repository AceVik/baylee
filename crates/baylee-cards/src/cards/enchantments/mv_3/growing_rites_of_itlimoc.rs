//! Growing Rites of Itlimoc // Itlimoc, Cradle of the Sun — {2}{G} — Legendary Enchantment // Legendary Land
//! Oracle: When Growing Rites of Itlimoc enters, look at the top four cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on the bottom of your library in any order.
//! Oracle: At the beginning of your end step, if you control four or more creatures, transform Growing Rites of Itlimoc.
//! Oracle: (Transforms from Growing Rites of Itlimoc.)
//! Oracle: {T}: Add {G}.
//! Oracle: {T}: Add {G} for each creature you control.
//! Set: LCI #188 — The Lost Caverns of Ixalan | Scryfall ID: 004524bf-b249-4dac-9c10-44d57143feb9 | Oracle ID: ea9c459a-6047-43aa-968f-a582be4000e8
//! Face: Growing Rites of Itlimoc — {2}{G} — Legendary Enchantment
//! Face: Itlimoc, Cradle of the Sun —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 585,
    oracle_id: "ea9c459a-6047-43aa-968f-a582be4000e8",
    scryfall_id: "004524bf-b249-4dac-9c10-44d57143feb9",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Growing Rites of Itlimoc",
        mana_cost: mana!("{2}{G}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Itlimoc, Cradle of the Sun",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
