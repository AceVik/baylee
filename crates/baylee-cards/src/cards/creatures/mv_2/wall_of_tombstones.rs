//! Wall of Tombstones — {1}{B} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: At the beginning of your upkeep, this creature's base toughness becomes equal to 1 plus the number of creature cards in your graveyard. (This effect lasts indefinitely.)
//! Set: LEG #129 — Legends | Scryfall ID: 55da1e86-fe18-486a-b510-f941e6f6e378 | Oracle ID: 2412a8a2-d028-44f1-8979-add9ec953759

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_TOMBSTONES,
    oracle_id = "2412a8a2-d028-44f1-8979-add9ec953759",
    scryfall_id = "55da1e86-fe18-486a-b510-f941e6f6e378",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wall of Tombstones",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::SetSourceToughness {
            base: Amount::CountOf {
                filter: &Filter::CREATURE,
                zone: ZoneSel::GraveyardYou
            },
            offset: 1,
            duration: Duration::Indefinitely
        }]
    ),],
);
