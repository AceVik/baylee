//! Wall of Caltrops — {1}{W} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Whenever this creature blocks a creature, if at least one other Wall creature is blocking that creature and no non-Wall creatures are blocking that creature, this creature gains banding until end of turn. (If any creatures with banding you control are blocking a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by.)
//! Set: LEG #42 — Legends | Scryfall ID: 664ad588-3002-4f63-93bd-38663171018f | Oracle ID: 20fa17d4-9c3e-470b-9993-a4a2799e33d1
// IMPLEMENTED — defender; whenever it blocks a creature that only Walls block (at least one other), it gains banding until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// A Wall creature.
const WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::WALL),
]);

/// A creature that is not a Wall.
const NON_WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

card!(
    index = index::WALL_OF_CALTROPS,
    oracle_id = "20fa17d4-9c3e-470b-9993-a4a2799e33d1",
    scryfall_id = "664ad588-3002-4f63-93bd-38663171018f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wall of Caltrops",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksCreature(&Filter::CREATURE),
        &[Effect::continuous(
            &Filter::This,
            Modifier::AddKeyword(KeywordSet::BANDING),
            Duration::UntilEndOfTurn,
        )],
        // "if at least one other Wall creature is blocking that creature and no
        // non-Wall creatures are blocking that creature" (CR 603.4).
        condition = Some(Condition::EventBlockedByOthers {
            some: &WALL,
            none: &NON_WALL,
        }),
    )],
);
