//! Original Seed Cradle: each life-gain event offers a paid, optional Plant.
use super::TOWER_SEEDLING;
use baylee_cards_dsl::prelude::*;

/// Common: pay three generic mana per life-gain event for one Plant.
pub static TOWER_CRADLE_COMMON: TokenDef = TokenDef {
    name: "Seed Cradle — Common",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::GainsLife(PlayerRel::You),
        &[Effect::PlayerMayPayThen {
            player: PlayerRel::You,
            mana: Amount::Fixed(3),
            effects: &[Effect::CreateTokensFor {
                player: PlayerRel::You,
                token: &TOWER_SEEDLING,
                amount: Amount::Fixed(1)
            }]
        }]
    )],
    ..TokenDef::DEFAULT
};

/// Uncommon: pay two generic mana per life-gain event for one Plant.
pub static TOWER_CRADLE_UNCOMMON: TokenDef = TokenDef {
    name: "Seed Cradle — Uncommon",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::GainsLife(PlayerRel::You),
        &[Effect::PlayerMayPayThen {
            player: PlayerRel::You,
            mana: Amount::Fixed(2),
            effects: &[Effect::CreateTokensFor {
                player: PlayerRel::You,
                token: &TOWER_SEEDLING,
                amount: Amount::Fixed(1)
            }]
        }]
    )],
    ..TokenDef::DEFAULT
};

/// Rare: pay one generic mana per life-gain event for one Plant.
pub static TOWER_CRADLE_RARE: TokenDef = TokenDef {
    name: "Seed Cradle — Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::GainsLife(PlayerRel::You),
        &[Effect::PlayerMayPayThen {
            player: PlayerRel::You,
            mana: Amount::Fixed(1),
            effects: &[Effect::CreateTokensFor {
                player: PlayerRel::You,
                token: &TOWER_SEEDLING,
                amount: Amount::Fixed(1)
            }]
        }]
    )],
    ..TokenDef::DEFAULT
};

/// Mythic Rare: pay one generic mana per life-gain event for two Plants.
pub static TOWER_CRADLE_MYTHIC: TokenDef = TokenDef {
    name: "Seed Cradle — Mythic Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[triggered!(
        Trigger::GainsLife(PlayerRel::You),
        &[Effect::PlayerMayPayThen {
            player: PlayerRel::You,
            mana: Amount::Fixed(1),
            effects: &[Effect::CreateTokensFor {
                player: PlayerRel::You,
                token: &TOWER_SEEDLING,
                amount: Amount::Fixed(2)
            }]
        }]
    )],
    ..TokenDef::DEFAULT
};
