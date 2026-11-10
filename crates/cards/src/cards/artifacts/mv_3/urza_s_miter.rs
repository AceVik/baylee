//! Urza's Miter — {3} — Artifact
//! Oracle: Whenever an artifact you control is put into a graveyard from the battlefield, if it wasn't sacrificed, you may pay {3}. If you do, draw a card.
//! Set: ME4 #237 — Masters Edition IV | Scryfall ID: 23270d99-5a25-4647-95f8-64da9b8e8831 | Oracle ID: 57f0c89c-79ad-4786-9497-c7e668620fc0

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URZA_S_MITER,
    oracle_id = "57f0c89c-79ad-4786-9497-c7e668620fc0",
    scryfall_id = "23270d99-5a25-4647-95f8-64da9b8e8831",
    faces = &[face!(
        name = "Urza's Miter",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::DiesNotSacrificed(&Filter::YOUR_ARTIFACT),
        &[Effect::PlayerMayPayThen {
            player: PlayerRel::You,
            mana: Amount::Fixed(3),
            effects: &[Effect::draw(1)]
        }],
    ),],
);
