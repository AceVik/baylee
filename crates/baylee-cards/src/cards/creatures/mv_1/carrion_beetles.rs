//! Carrion Beetles — {B} — Creature — Insect
//! Oracle: {2}{B}, {T}: Exile up to three target cards from a single graveyard.
//! Set: USG #122 — Urza's Saga | Scryfall ID: 46d4f4d7-a35b-45ea-ba51-ee65b3ff98d4 | Oracle ID: d64a51be-2dd1-485f-98fa-c93d4c296e68
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CARRION_BEETLES,
    oracle_id = "d64a51be-2dd1-485f-98fa-c93d4c296e68",
    scryfall_id = "46d4f4d7-a35b-45ea-ba51-ee65b3ff98d4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Carrion Beetles",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{2}{B}", TapSelf),
        &[Effect::exile(TargetSpec::CardInGraveyard(
            &Filter::Any,
            PlayerRel::Chosen,
        ))],
        targets = Some(TargetReq::up_to(
            TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::Chosen),
            3,
        )),
    )],
);
