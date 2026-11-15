//! Original Hearthstone: your creature entries, including tokens, gain life.
use baylee_cards_dsl::prelude::*;

/// Common: gain one life per entering creature you control.
pub static TOWER_HEARTH_COMMON: TokenDef = TokenDef {
    name: "Hearthstone — Common",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::EntersBattlefield(&Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])),
        &[Effect::gain_life(1)]
    )],
    ..TokenDef::DEFAULT
};

/// Uncommon: gain two life per entering creature you control.
pub static TOWER_HEARTH_UNCOMMON: TokenDef = TokenDef {
    name: "Hearthstone — Uncommon",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::EntersBattlefield(&Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])),
        &[Effect::gain_life(2)]
    )],
    ..TokenDef::DEFAULT
};

/// Rare: gain three life per entering creature you control.
pub static TOWER_HEARTH_RARE: TokenDef = TokenDef {
    name: "Hearthstone — Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::EntersBattlefield(&Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])),
        &[Effect::gain_life(3)]
    )],
    ..TokenDef::DEFAULT
};

/// Mythic Rare: gain four life per entering creature you control.
pub static TOWER_HEARTH_MYTHIC: TokenDef = TokenDef {
    name: "Hearthstone — Mythic Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::EntersBattlefield(&Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])),
        &[Effect::gain_life(4)]
    )],
    ..TokenDef::DEFAULT
};
