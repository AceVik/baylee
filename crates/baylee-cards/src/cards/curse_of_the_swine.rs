//! Curse of the Swine — {X}{U}{U} — Sorcery
//! Oracle: Exile X target creatures. For each creature exiled this way, its controller creates a 2/2 green Boar creature token.
//! Set: SOC #192 — Secrets of Strixhaven Commander | Scryfall ID: 91eb9067-0bc7-4497-ba9c-c1ea41e5a379 | Oracle ID: 5669ea7c-c4fc-494c-896b-4bce9b494817
// IMPLEMENTED — X-cost mass exile with per-controller Boar tokens.

use baylee_cards_dsl::prelude::*;

use crate::tokens::BOAR_2_2_GREEN as BOAR_TOKEN;

card! {
    index: 27,
    oracle_id: "5669ea7c-c4fc-494c-896b-4bce9b494817",
    scryfall_id: "91eb9067-0bc7-4497-ba9c-c1ea41e5a379",
    faces: &[face! {
        name: "Curse of the Swine",
        mana_cost: baylee_core::mana!("{X}{U}{U}"),
        types: TypeSet::SORCERY,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::ExileTargetsCreateTokens { token: &BOAR_TOKEN }], targets: Some(TargetReq::x_targets(TargetSpec::Object(&Filter::CREATURE))))],
}

// X creatures exiled; each controller gets a Boar per exiled creature.
