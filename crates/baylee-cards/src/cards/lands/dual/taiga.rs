//! Taiga — (no cost) — Land — MOUNTAIN FOREST
//! Oracle: ({T}: Add {R} or {G}.)
//! Set: VMA #317 — Vintage Masters | Scryfall ID: 0c2c39fc-b564-4ab5-833c-ff029760b7a7 | Oracle ID: 22e3cf1d-3559-4ce1-954c-8dc815342979
// IMPLEMENTED — two-color mana choice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static COLORS: &[ManaColor] = &[ManaColor::Red, ManaColor::Green];
static SUBS: &[SubtypeId] = &[land::MOUNTAIN, land::FOREST];

card! {
    index: 165,
    oracle_id: "22e3cf1d-3559-4ce1-954c-8dc815342979",
    scryfall_id: "0c2c39fc-b564-4ab5-833c-ff029760b7a7",
    faces: &[face! {
        name: "Taiga",
        types: TypeSet::LAND,
        subtypes: SUBS,
    }],
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(COLORS)])],
}
