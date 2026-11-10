//! Essence Filter — {1}{G}{G} — Sorcery
//! Oracle: Destroy all enchantments or all nonwhite enchantments.
//! Set: ME2 #160 — Masters Edition II | Scryfall ID: dcd5fdd0-8f33-4e98-8c21-f62c5f4c89c9 | Oracle ID: b9f63388-d1ac-4895-84c5-e5b1db187463
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const NONWHITE_ENCHANTMENT: Filter = Filter::And(&[
    Filter::ENCHANTMENT,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::White]))),
]);

card!(
    index = index::ESSENCE_FILTER,
    oracle_id = "b9f63388-d1ac-4895-84c5-e5b1db187463",
    scryfall_id = "dcd5fdd0-8f33-4e98-8c21-f62c5f4c89c9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Essence Filter",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::ChooseOne {
        choices: &[
            Effect::DestroyAll {
                filter: &Filter::ENCHANTMENT,
                no_regen: false,
            },
            Effect::DestroyAll {
                filter: &NONWHITE_ENCHANTMENT,
                no_regen: false,
            },
        ],
    }])],
);
