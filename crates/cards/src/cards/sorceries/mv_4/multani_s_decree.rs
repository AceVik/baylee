//! Multani's Decree — {3}{G} — Sorcery
//! Oracle: Destroy all enchantments. You gain 2 life for each enchantment destroyed this way.
//! Set: UDS #114 — Urza's Destiny | Scryfall ID: 58b4d5c8-23fc-4fb8-99d6-bb64e66cc4db | Oracle ID: b233473d-d22b-45e4-9fe7-55d1170a788b
// IMPLEMENTED — destroys all enchantments, then its controller gains 2 life for each enchantment actually destroyed, including exile-replaced destruction.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MULTANI_S_DECREE,
    oracle_id = "b233473d-d22b-45e4-9fe7-55d1170a788b",
    scryfall_id = "58b4d5c8-23fc-4fb8-99d6-bb64e66cc4db",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Multani's Decree",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::destroy_all(&Filter::ENCHANTMENT),
        Effect::GainLife {
            amount: Amount::Times {
                base: &Amount::ThisWay(ThisWay::Destroyed(&Filter::ENCHANTMENT)),
                factor: 2,
            },
        },
    ])],
);
