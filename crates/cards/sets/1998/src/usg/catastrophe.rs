//! Catastrophe — {4}{W}{W} — Sorcery
//! Oracle: Destroy all lands or all creatures. Creatures destroyed this way can't be regenerated.
//! Set: USG #6 — Urza's Saga | Scryfall ID: 294d21dc-5c76-4449-936f-9b7541d37c86 | Oracle ID: ed68160e-5e3d-44f2-ac7a-3749fda10119
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CATASTROPHE,
    oracle_id = "ed68160e-5e3d-44f2-ac7a-3749fda10119",
    scryfall_id = "294d21dc-5c76-4449-936f-9b7541d37c86",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Catastrophe",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::ChooseOne {
        choices: &[
            Effect::DestroyAll {
                filter: &Filter::LAND,
                no_regen: true,
            },
            Effect::DestroyAll {
                filter: &Filter::CREATURE,
                no_regen: true,
            },
        ],
    }])],
);
