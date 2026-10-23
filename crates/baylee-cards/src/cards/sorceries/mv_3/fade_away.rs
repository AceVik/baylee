//! Fade Away — {2}{U} — Sorcery
//! Oracle: For each creature, its controller sacrifices a permanent of their choice unless they pay {1}.
//! Set: EXO #34 — Exodus | Scryfall ID: a6f9103e-dcc2-4f7a-a8ca-eaa831f5f83b | Oracle ID: f92c2d47-f2f7-43d9-acf3-0145bad1977d
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FADE_AWAY,
    oracle_id = "f92c2d47-f2f7-43d9-acf3-0145bad1977d",
    scryfall_id = "a6f9103e-dcc2-4f7a-a8ca-eaa831f5f83b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fade Away",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::ForEachMatching {
        filter: &Filter::CREATURE,
        effects: &[Effect::PlayerMayPayOr {
            player: PlayerRel::EventPlayer,
            mana: Amount::Fixed(1),
            effect: &Effect::SacrificeFilter {
                who: PlayerRel::EventPlayer,
                filter: &Filter::Any,
            },
        }],
    }])],
);
