//! Planar Birth — {1}{W} — Sorcery
//! Oracle: Return all basic land cards from all graveyards to the battlefield tapped under their owners' control.
//! Set: USG #31 — Urza's Saga | Scryfall ID: c7cacdff-aa83-4644-b2f0-ce8c89dddfbf | Oracle ID: 7a9d13b3-3823-463a-acd9-8b7a9d5e121f
// PARTIAL: returning basic land cards from all graveyards to the battlefield tapped under their owners' control is not in Effect; each player returns all basic land cards from their graveyard to the battlefield untapped under their control.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANAR_BIRTH,
    oracle_id = "7a9d13b3-3823-463a-acd9-8b7a9d5e121f",
    scryfall_id = "c7cacdff-aa83-4644-b2f0-ce8c89dddfbf",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "returning basic land cards from all graveyards to the battlefield tapped under their owners' control is not in Effect; each player returns all basic land cards from their graveyard to the battlefield untapped under their control",
    ),
    faces = &[face!(
        name = "Planar Birth",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::EachPlayerGraveyardToBattlefield {
        filter: &Filter::BASIC_LAND,
    }]),],
);
