//! Titania's Boon — {3}{G} — Sorcery
//! Oracle: Put a +1/+1 counter on each creature you control.
//! Set: USG #276 — Urza's Saga | Scryfall ID: b8d64591-8552-4e94-b932-7a23922513a1 | Oracle ID: 0bf4c01d-3cb0-47be-b84a-6a399bc2d3fc
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TITANIA_S_BOON,
    oracle_id = "0bf4c01d-3cb0-47be-b84a-6a399bc2d3fc",
    scryfall_id = "b8d64591-8552-4e94-b932-7a23922513a1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Titania's Boon",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::AddCounterFilter {
        filter: &Filter::YOUR_CREATURE,
        kind: CounterKind::P1P1,
        amount: Amount::Fixed(1),
    }]),],
);
