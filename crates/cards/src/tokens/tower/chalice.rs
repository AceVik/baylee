//! Original Spring Chalice: recurring life, through an ordinary upkeep trigger.
use baylee_cards_dsl::prelude::*;

/// Common: gain one life at your upkeep.
pub static TOWER_CHALICE_COMMON: TokenDef = TokenDef {
    name: "Spring Chalice — Common",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::gain_life(1)]
    )],
    ..TokenDef::DEFAULT
};

/// Uncommon: gain two life at your upkeep.
pub static TOWER_CHALICE_UNCOMMON: TokenDef = TokenDef {
    name: "Spring Chalice — Uncommon",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::gain_life(2)]
    )],
    ..TokenDef::DEFAULT
};

/// Rare: gain three life at your upkeep.
pub static TOWER_CHALICE_RARE: TokenDef = TokenDef {
    name: "Spring Chalice — Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::gain_life(3)]
    )],
    ..TokenDef::DEFAULT
};

/// Mythic Rare: gain five life at your upkeep.
pub static TOWER_CHALICE_MYTHIC: TokenDef = TokenDef {
    name: "Spring Chalice — Mythic Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::gain_life(5)]
    )],
    ..TokenDef::DEFAULT
};
