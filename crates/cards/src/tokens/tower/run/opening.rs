//! Opening-resource equipment and the Living Grove. Setup bonuses are pack-bound.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Common Heartwood: its starting-life bonus is derived by the native pack.
pub static TOWER_RUN_HEARTWOOD_COMMON: TokenDef = TokenDef {
    name: "Heartwood Charm — Common",
    ..TokenDef::DEFAULT
};
/// Uncommon adds upkeep healing.
pub static TOWER_RUN_HEARTWOOD_UNCOMMON: TokenDef = TokenDef {
    name: "Heartwood Charm — Uncommon",
    abilities: &[UPKEEP_LIFE],
    ..TokenDef::DEFAULT
};
/// Rare additionally heals when your creatures enter.
pub static TOWER_RUN_HEARTWOOD_RARE: TokenDef = TokenDef {
    name: "Heartwood Charm — Rare",
    abilities: &[UPKEEP_LIFE, ENTER_LIFE],
    ..TokenDef::DEFAULT
};
/// Mythic turns each life-gain event into opponent life loss.
pub static TOWER_RUN_HEARTWOOD_MYTHIC: TokenDef = TokenDef {
    name: "Heartwood Charm — Mythic Rare",
    abilities: &[UPKEEP_LIFE, ENTER_LIFE, LIFE_DRAIN],
    ..TokenDef::DEFAULT
};

/// Common Satchel: its hand bonuses are derived by the native pack.
pub static TOWER_RUN_SATCHEL_COMMON: TokenDef = TokenDef {
    name: "Sage's Satchel — Common",
    ..TokenDef::DEFAULT
};
/// Uncommon adds upkeep selection.
pub static TOWER_RUN_SATCHEL_UNCOMMON: TokenDef = TokenDef {
    name: "Sage's Satchel — Uncommon",
    abilities: &[UPKEEP_SCRY],
    ..TokenDef::DEFAULT
};
/// Rare offers a paid once-per-turn loot.
pub static TOWER_RUN_SATCHEL_RARE: TokenDef = TokenDef {
    name: "Sage's Satchel — Rare",
    abilities: &[UPKEEP_SCRY, LOOT],
    ..TokenDef::DEFAULT
};
/// Mythic heals with every draw.
pub static TOWER_RUN_SATCHEL_MYTHIC: TokenDef = TokenDef {
    name: "Sage's Satchel — Mythic Rare",
    abilities: &[UPKEEP_SCRY, LOOT, DRAW_LIFE],
    ..TokenDef::DEFAULT
};

const FILTER_MANA: AbilityDef = mana(cost!("{1}"), &[Effect::mana_of_any_color()], ONCE);
const FIRST_SPELL_DRAW: AbilityDef = triggered!(
    Trigger::FirstNoncreatureSpellCast(PlayerRel::You),
    &[Effect::draw(1)]
);
/// Common Compass: its free mulligans are derived by the native pack.
pub static TOWER_RUN_COMPASS_COMMON: TokenDef = TokenDef {
    name: "Wayfinder's Compass — Common",
    ..TokenDef::DEFAULT
};
/// Uncommon adds upkeep selection.
pub static TOWER_RUN_COMPASS_UNCOMMON: TokenDef = TokenDef {
    name: "Wayfinder's Compass — Uncommon",
    abilities: &[UPKEEP_SCRY],
    ..TokenDef::DEFAULT
};
/// Rare filters one generic mana each turn.
pub static TOWER_RUN_COMPASS_RARE: TokenDef = TokenDef {
    name: "Wayfinder's Compass — Rare",
    abilities: &[UPKEEP_SCRY, FILTER_MANA],
    ..TokenDef::DEFAULT
};
/// Mythic rewards the first noncreature spell on each turn.
pub static TOWER_RUN_COMPASS_MYTHIC: TokenDef = TokenDef {
    name: "Wayfinder's Compass — Mythic Rare",
    abilities: &[UPKEEP_SCRY, FILTER_MANA, FIRST_SPELL_DRAW],
    ..TokenDef::DEFAULT
};

/// Common Grove creates one Plant each upkeep.
pub static TOWER_RUN_GROVE_COMMON: TokenDef = TokenDef {
    name: "Living Grove — Common",
    abilities: &[super::super::TOWER_GROVE_COMMON.abilities[0]],
    ..TokenDef::DEFAULT
};
/// Uncommon creates two Plants and heals as your creature tokens enter.
pub static TOWER_RUN_GROVE_UNCOMMON: TokenDef = TokenDef {
    name: "Living Grove — Uncommon",
    abilities: &[super::super::TOWER_GROVE_UNCOMMON.abilities[0], TOKEN_LIFE],
    ..TokenDef::DEFAULT
};
/// Rare creates three Plants and grows your creature tokens.
pub static TOWER_RUN_GROVE_RARE: TokenDef = TokenDef {
    name: "Living Grove — Rare",
    abilities: &[
        super::super::TOWER_GROVE_RARE.abilities[0],
        TOKEN_LIFE,
        TOKEN_GROWTH,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic creates four Plants and grants your tokens trample.
pub static TOWER_RUN_GROVE_MYTHIC: TokenDef = TokenDef {
    name: "Living Grove — Mythic Rare",
    abilities: &[
        super::super::TOWER_GROVE_MYTHIC.abilities[0],
        TOKEN_LIFE,
        TOKEN_GROWTH,
        TOKEN_TRAMPLE,
    ],
    ..TokenDef::DEFAULT
};
