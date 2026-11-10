//! Wall of Dust — {2}{R} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Whenever this creature blocks a creature, that creature can't attack during its controller's next turn.
//! Set: 4ED #229 — Fourth Edition | Scryfall ID: b25b81fb-1d0f-4c0c-8d54-fbf9c1d54578 | Oracle ID: 14895574-9c87-4e80-9bc9-cc3dd22a42b8
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_DUST,
    oracle_id = "14895574-9c87-4e80-9bc9-cc3dd22a42b8",
    scryfall_id = "b25b81fb-1d0f-4c0c-8d54-fbf9c1d54578",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wall of Dust",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(1),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksCreature(&Filter::CREATURE),
        &[Effect::CantAttackDuringNextControllerTurn {
            target: TargetSpec::EventObject
        }]
    ),],
);
