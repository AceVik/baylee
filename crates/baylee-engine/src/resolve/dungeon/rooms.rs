//! Printed room bodies, executed as separate triggered abilities.
use super::{
    Amount, ColorSet, DUNGEON_TREASURE, Dungeon, Duration, Effect, Filter, KeywordSet, Modifier,
    PlayerRel, TargetSpec,
};
use baylee_cards_dsl::{CounterKind, Find, TokenDef};
use baylee_core::{
    color::Color,
    generated::subtypes::creature,
    types::{SupertypeSet, TypeSet},
};

static GOBLIN: TokenDef = TokenDef {
    name: "Goblin",
    types: TypeSet::CREATURE,
    subtypes: &[creature::GOBLIN],
    colors: ColorSet::of(Color::Red),
    power: Some(1),
    toughness: Some(1),
    ..TokenDef::DEFAULT
};
static SKELETON: TokenDef = TokenDef {
    name: "Skeleton",
    types: TypeSet::CREATURE,
    subtypes: &[creature::SKELETON],
    colors: ColorSet::of(Color::Black),
    power: Some(1),
    toughness: Some(1),
    ..TokenDef::DEFAULT
};
static ATROPAL: TokenDef = TokenDef {
    name: "The Atropal",
    types: TypeSet::CREATURE,
    subtypes: &[creature::GOD, creature::HORROR],
    supertypes: SupertypeSet::LEGENDARY,
    colors: ColorSet::of(Color::Black),
    power: Some(4),
    toughness: Some(4),
    keywords: KeywordSet::DEATHTOUCH,
    ..TokenDef::DEFAULT
};
static CATACOMBS_SKELETON: TokenDef = TokenDef {
    name: "Skeleton",
    types: TypeSet::CREATURE,
    subtypes: &[creature::SKELETON],
    colors: ColorSet::of(Color::Black),
    power: Some(4),
    toughness: Some(1),
    keywords: KeywordSet::MENACE,
    ..TokenDef::DEFAULT
};

static ROOM_0: &[Effect] = &[Effect::scry(1)];
static ROOM_1: &[Effect] = &[Effect::CreateToken { token: &GOBLIN }];
static ROOM_2: &[Effect] = &[Effect::CreateToken {
    token: &DUNGEON_TREASURE,
}];
static ROOM_3: &[Effect] = &[Effect::AddCounter {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(1),
}];
static ROOM_4: &[Effect] = &[
    Effect::LoseLife {
        amount: Amount::Fixed(1),
        target: PlayerRel::EachOpponent,
    },
    Effect::gain_life(1),
];
static ROOM_5: &[Effect] = &[Effect::PumpTarget {
    power: Amount::NegXFixed(4),
    toughness: Amount::Fixed(0),
    keywords: KeywordSet::EMPTY,
    duration: Duration::UntilYourNextTurn,
}];
static ROOM_6: &[Effect] = &[Effect::draw(1)];
static ROOM_7: &[Effect] = &[Effect::gain_life(1)];
static ROOM_8: &[Effect] = &[Effect::ModifyTargets {
    modifier: Modifier::CantAttack,
    duration: Duration::UntilYourNextTurn,
}];
static ROOM_9: &[Effect] = &[Effect::scry(2)];
static ROOM_10: &[Effect] = &[Effect::DungeonExileTwo];
static ROOM_11: &[Effect] = &[
    Effect::CreateToken { token: &SKELETON },
    Effect::CreateToken { token: &SKELETON },
];
static ROOM_12: &[Effect] = &[Effect::scry(3)];
static ROOM_13: &[Effect] = &[Effect::DungeonDrawThree];
static ROOM_14: &[Effect] = &[Effect::LoseLife {
    amount: Amount::Fixed(1),
    target: PlayerRel::EachPlayer,
}];
static ROOM_15: &[Effect] = &[Effect::DungeonTax { discard: true }];
static ROOM_16: &[Effect] = &[Effect::DungeonTax { discard: false }];
static ROOM_17: &[Effect] = &[
    Effect::DiscardForPlayers {
        who: PlayerRel::You,
        count: 1,
    },
    Effect::DungeonSacrificeThree,
];
static ROOM_18: &[Effect] = &[Effect::CreateToken { token: &ATROPAL }];
static UNDERCITY_SECRET_ENTRANCE: &[Effect] = &[Effect::SearchLibrary {
    filter: &Filter::BASIC_LAND,
    finds: &[Find::HAND],
    optional: false,
}];
static UNDERCITY_FORGE: &[Effect] = &[Effect::AddCounter {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(2),
}];
static UNDERCITY_TRAP: &[Effect] = &[Effect::LoseLife {
    amount: Amount::Fixed(5),
    target: PlayerRel::TargetPlayerOrController,
}];
static UNDERCITY_ARENA: &[Effect] = &[Effect::Goad {
    target: TargetSpec::Object(&Filter::CREATURE),
}];
static UNDERCITY_CATACOMBS: &[Effect] = &[Effect::CreateToken {
    token: &CATACOMBS_SKELETON,
}];
static UNDERCITY_THRONE: &[Effect] = &[Effect::DungeonThroneOfTheDeadThree];

pub(super) fn effects(dungeon: Dungeon, room: u8) -> (&'static [Effect], Option<TargetSpec>) {
    let creature = Some(TargetSpec::Object(&Filter::CREATURE));
    match (dungeon, room) {
        (Dungeon::LostMine, 0) | (Dungeon::MadMage, 1) => (ROOM_0, None),
        (Dungeon::LostMine, 1) => (ROOM_1, None),
        (Dungeon::LostMine | Dungeon::MadMage, 2) | (Dungeon::Undercity, 5) => (ROOM_2, None),
        (Dungeon::LostMine, 3) => (ROOM_3, creature),
        (Dungeon::LostMine, 4) => (ROOM_4, None),
        (Dungeon::LostMine, 5) => (ROOM_5, creature),
        (Dungeon::LostMine | Dungeon::Undercity, 6) => (ROOM_6, None),
        (Dungeon::MadMage, 0) => (ROOM_7, None),
        (Dungeon::MadMage, 3) => (ROOM_8, creature),
        (Dungeon::MadMage, 4) | (Dungeon::Undercity, 2) => (ROOM_9, None),
        (Dungeon::MadMage, 5) => (ROOM_10, None),
        (Dungeon::MadMage, 6) => (ROOM_11, None),
        (Dungeon::MadMage, 7) => (ROOM_12, None),
        (Dungeon::MadMage, 8) => (ROOM_13, None),
        (Dungeon::Tomb, 0) => (ROOM_14, None),
        (Dungeon::Tomb, 1) => (ROOM_15, None),
        (Dungeon::Tomb, 2) => (ROOM_16, None),
        (Dungeon::Tomb, 3) => (ROOM_17, None),
        (Dungeon::Tomb, 4) => (ROOM_18, None),
        (Dungeon::Undercity, 0) => (UNDERCITY_SECRET_ENTRANCE, None),
        (Dungeon::Undercity, 1) => (UNDERCITY_FORGE, creature),
        (Dungeon::Undercity, 3) => (UNDERCITY_TRAP, Some(TargetSpec::AnyPlayer)),
        (Dungeon::Undercity, 4) => (UNDERCITY_ARENA, creature),
        (Dungeon::Undercity, 7) => (UNDERCITY_CATACOMBS, None),
        (Dungeon::Undercity, 8) => (UNDERCITY_THRONE, None),
        _ => unreachable!("offered dungeon room"),
    }
}
