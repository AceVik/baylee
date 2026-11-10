//! Dark Maze — {4}{U} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {0}: This creature can attack this turn as though it didn't have defender. Exile it at the beginning of the next end step.
//! Set: 5ED #80 — Fifth Edition | Scryfall ID: 74fe0349-a114-4bb8-a9ae-97d0f8f46ddb | Oracle ID: 44c65424-9c44-4bd8-9022-7ef5bb5b898e
// IMPLEMENTED — defender, and {0} to attack this turn as though it didn't have defender and exile it at the beginning of the next end step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DARK_MAZE,
    oracle_id = "44c65424-9c44-4bd8-9022-7ef5bb5b898e",
    scryfall_id = "74fe0349-a114-4bb8-a9ae-97d0f8f46ddb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dark Maze",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(4),
        toughness = Some(5),
    ),],
    abilities = &[activated!(
        Cost::FREE,
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::AttacksDespiteDefender,
                Duration::UntilEndOfTurn,
            ),
            Effect::AtNextEndStep {
                effects: &[Effect::exile(TargetSpec::EventObject)],
            },
        ]
    )],
);
