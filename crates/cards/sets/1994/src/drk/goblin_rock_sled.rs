//! Goblin Rock Sled — {1}{R} — Creature — Goblin
//! Oracle: Trample
//! Oracle: This creature doesn't untap during your untap step if it attacked during your last turn.
//! Oracle: This creature can't attack unless defending player controls a Mountain.
//! Set: 4ED #199 — Fourth Edition | Scryfall ID: 25f4f0c1-a43e-4282-b20e-91c711c57521 | Oracle ID: ada3247e-ec5e-499d-bc15-1d9dd80a59ae

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const MOUNTAIN: Filter = Filter::HasSubtype(subtypes::land::MOUNTAIN);

card!(
    index = index::GOBLIN_ROCK_SLED,
    oracle_id = "ada3247e-ec5e-499d-bc15-1d9dd80a59ae",
    scryfall_id = "25f4f0c1-a43e-4282-b20e-91c711c57521",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Rock Sled",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(3),
        toughness = Some(1),
        keywords = KeywordSet::TRAMPLE,
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::DoesNotUntap,
            condition = Some(Condition::SourceAttackedDuringYourLastTurn),
        ),
        static_ability!(
            Filter::This,
            Modifier::CantAttackUnlessDefenderControls(&MOUNTAIN),
        ),
    ],
);
