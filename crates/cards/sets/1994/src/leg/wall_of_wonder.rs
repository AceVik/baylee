//! Wall of Wonder — {2}{U}{U} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {2}{U}{U}: This creature gets +4/-4 until end of turn and can attack this turn as though it didn't have defender.
//! Set: 7ED #112 — Seventh Edition | Scryfall ID: e8c73e58-e906-4c67-9f84-b20456629cb0 | Oracle ID: 10889b44-d031-413c-a8c4-d6c13c400e01
// IMPLEMENTED — defender, and {2}{U}{U} gives it +4/-4 and the ability to
// attack this turn as though it didn't have defender.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_WONDER,
    oracle_id = "10889b44-d031-413c-a8c4-d6c13c400e01",
    scryfall_id = "e8c73e58-e906-4c67-9f84-b20456629cb0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wall of Wonder",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(1),
        toughness = Some(5),
    ),],
    abilities = &[activated!(
        cost!("{2}{U}{U}"),
        &[
            Effect::PumpFilter {
                filter: &Filter::This,
                controlled_by: None,
                power: Amount::Fixed(4),
                toughness: Amount::NegXFixed(4),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::continuous(
                &Filter::This,
                Modifier::AttacksDespiteDefender,
                Duration::UntilEndOfTurn,
            ),
        ],
    )],
);
