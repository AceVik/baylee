//! Grave Robbers — {1}{B}{B} — Creature — Human Rogue
//! Oracle: {B}, {T}: Exile target artifact card from a graveyard. You gain 2 life.
//! Set: DRK #46 — The Dark | Scryfall ID: a131605a-f646-4745-a1e4-48d155a3d94f | Oracle ID: 6698b059-f1ed-4023-9d44-fbdf39fb159a

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRAVE_ROBBERS,
    oracle_id = "6698b059-f1ed-4023-9d44-fbdf39fb159a",
    scryfall_id = "a131605a-f646-4745-a1e4-48d155a3d94f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Grave Robbers",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{B}", TapSelf),
        &[
            Effect::Exile {
                target: TargetSpec::CardInGraveyard(&Filter::ARTIFACT, PlayerRel::EachPlayer),
            },
            Effect::GainLife {
                amount: Amount::Fixed(2),
            },
        ],
        target = Some(TargetSpec::CardInGraveyard(
            &Filter::ARTIFACT,
            PlayerRel::EachPlayer,
        )),
    )],
);
