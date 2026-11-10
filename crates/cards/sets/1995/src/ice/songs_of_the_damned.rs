//! Songs of the Damned — {B} — Instant
//! Oracle: Add {B} for each creature card in your graveyard.
//! Set: UMA #115 — Ultimate Masters | Scryfall ID: 9133b267-295d-4987-b1d6-f32a85b66081 | Oracle ID: 39419d29-47e7-4d71-953b-96a2e14ae964

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SONGS_OF_THE_DAMNED,
    oracle_id = "39419d29-47e7-4d71-953b-96a2e14ae964",
    scryfall_id = "9133b267-295d-4987-b1d6-f32a85b66081",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Songs of the Damned",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        // CR 106.1: Add {B} for each creature card in your graveyard.
        spell!(&[Effect::mana_dynamic(
            ManaColor::Black,
            Amount::CountOf {
                filter: &Filter::CREATURE,
                zone: ZoneSel::GraveyardYou,
            },
        )]),
    ],
);
