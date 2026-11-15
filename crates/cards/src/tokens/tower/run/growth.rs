//! Renewable token engines. Prices are paid; legal player combinations have no cap.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Common offers one Plant for three mana after each life-gain event.
pub static TOWER_RUN_CRADLE_COMMON: TokenDef = TokenDef {
    name: "Seed Cradle — Common",
    abilities: &[super::super::TOWER_CRADLE_COMMON.abilities[0]],
    ..TokenDef::DEFAULT
};
/// Uncommon lowers the price to two and gains life each upkeep.
pub static TOWER_RUN_CRADLE_UNCOMMON: TokenDef = TokenDef {
    name: "Seed Cradle — Uncommon",
    abilities: &[
        super::super::TOWER_CRADLE_UNCOMMON.abilities[0],
        UPKEEP_LIFE,
    ],
    ..TokenDef::DEFAULT
};
/// Rare lowers the price to one and toughens creature tokens.
pub static TOWER_RUN_CRADLE_RARE: TokenDef = TokenDef {
    name: "Seed Cradle — Rare",
    abilities: &[
        super::super::TOWER_CRADLE_RARE.abilities[0],
        UPKEEP_LIFE,
        TOKEN_GUARD,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic offers two Plants per payment and grants tokens haste.
pub static TOWER_RUN_CRADLE_MYTHIC: TokenDef = TokenDef {
    name: "Seed Cradle — Mythic Rare",
    abilities: &[
        super::super::TOWER_CRADLE_MYTHIC.abilities[0],
        UPKEEP_LIFE,
        TOKEN_GUARD,
        TOKEN_HASTE,
    ],
    ..TokenDef::DEFAULT
};

/// Common gives your creature tokens +1/+0.
pub static TOWER_RUN_BANNER_COMMON: TokenDef = TokenDef {
    name: "Swarm Banner — Common",
    abilities: &[super::super::TOWER_BANNER_COMMON.abilities[0]],
    ..TokenDef::DEFAULT
};
/// Uncommon gives +1/+1 and vigilance.
pub static TOWER_RUN_BANNER_UNCOMMON: TokenDef = TokenDef {
    name: "Swarm Banner — Uncommon",
    abilities: &[
        super::super::TOWER_BANNER_UNCOMMON.abilities[0],
        TOKEN_VIGILANCE,
    ],
    ..TokenDef::DEFAULT
};
/// Rare gives +2/+1 and adds trample.
pub static TOWER_RUN_BANNER_RARE: TokenDef = TokenDef {
    name: "Swarm Banner — Rare",
    abilities: &[
        super::super::TOWER_BANNER_RARE.abilities[0],
        TOKEN_VIGILANCE,
        TOKEN_TRAMPLE,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic gives +2/+2 and adds lifelink.
pub static TOWER_RUN_BANNER_MYTHIC: TokenDef = TokenDef {
    name: "Swarm Banner — Mythic Rare",
    abilities: &[
        super::super::TOWER_BANNER_MYTHIC.abilities[0],
        TOKEN_VIGILANCE,
        TOKEN_TRAMPLE,
        TOKEN_LIFELINK,
    ],
    ..TokenDef::DEFAULT
};

const DEATH_CHARGE: AbilityDef = triggered!(Trigger::Dies(&f!(your token CREATURE)), &[CHARGE_ONE]);
const ENTER_CHARGE: AbilityDef = triggered!(
    Trigger::EntersBattlefield(&f!(your token CREATURE)),
    &[CHARGE_ONE]
);
const THREE: Cost = cost!(RemoveCounterSelf {
    kind: CounterKind::Charge,
    n: 3
});
const TWO: Cost = cost!(RemoveCounterSelf {
    kind: CounterKind::Charge,
    n: 2
});
const ONE: Cost = cost!(RemoveCounterSelf {
    kind: CounterKind::Charge,
    n: 1
});

/// Common stores token deaths; three charges buy two colorless mana once per turn.
pub static TOWER_RUN_HARVEST_COMMON: TokenDef = TokenDef {
    name: "Harvest Wheel — Common",
    abilities: &[
        DEATH_CHARGE,
        mana(THREE, &[Effect::mana(ManaColor::Colorless, 2)], ONCE),
    ],
    ..TokenDef::DEFAULT
};
/// Uncommon needs two charges and gains life when your creatures die.
pub static TOWER_RUN_HARVEST_UNCOMMON: TokenDef = TokenDef {
    name: "Harvest Wheel — Uncommon",
    abilities: &[
        DEATH_CHARGE,
        mana(TWO, &[Effect::mana(ManaColor::Colorless, 2)], ONCE),
        DEATH_LIFE,
    ],
    ..TokenDef::DEFAULT
};
/// Rare needs one charge and grows a Plant each upkeep.
pub static TOWER_RUN_HARVEST_RARE: TokenDef = TokenDef {
    name: "Harvest Wheel — Rare",
    abilities: &[
        DEATH_CHARGE,
        mana(ONE, &[Effect::mana(ManaColor::Colorless, 2)], ONCE),
        DEATH_LIFE,
        UPKEEP_PLANT,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic has no activation cap, yields three mana and grants token haste.
pub static TOWER_RUN_HARVEST_MYTHIC: TokenDef = TokenDef {
    name: "Harvest Wheel — Mythic Rare",
    abilities: &[
        DEATH_CHARGE,
        mana(ONE, &[Effect::mana(ManaColor::Colorless, 3)], UNLIMITED),
        DEATH_LIFE,
        UPKEEP_PLANT,
        TOKEN_HASTE,
    ],
    ..TokenDef::DEFAULT
};

/// Common stores token entries; three charges buy green mana once per turn.
pub static TOWER_RUN_ROOTS_COMMON: TokenDef = TokenDef {
    name: "Root Reservoir — Common",
    abilities: &[
        ENTER_CHARGE,
        mana(THREE, &[Effect::mana(ManaColor::Green, 1)], ONCE),
    ],
    ..TokenDef::DEFAULT
};
/// Uncommon needs two charges and grows a Plant each upkeep.
pub static TOWER_RUN_ROOTS_UNCOMMON: TokenDef = TokenDef {
    name: "Root Reservoir — Uncommon",
    abilities: &[
        ENTER_CHARGE,
        mana(TWO, &[Effect::mana(ManaColor::Green, 1)], ONCE),
        UPKEEP_PLANT,
    ],
    ..TokenDef::DEFAULT
};
/// Rare removes the turn cap and gains life when your creatures die.
pub static TOWER_RUN_ROOTS_RARE: TokenDef = TokenDef {
    name: "Root Reservoir — Rare",
    abilities: &[
        ENTER_CHARGE,
        mana(TWO, &[Effect::mana(ManaColor::Green, 1)], UNLIMITED),
        UPKEEP_PLANT,
        DEATH_LIFE,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic needs one charge and strengthens creature tokens.
pub static TOWER_RUN_ROOTS_MYTHIC: TokenDef = TokenDef {
    name: "Root Reservoir — Mythic Rare",
    abilities: &[
        ENTER_CHARGE,
        mana(ONE, &[Effect::mana(ManaColor::Green, 1)], UNLIMITED),
        UPKEEP_PLANT,
        DEATH_LIFE,
        TOKEN_GROWTH,
    ],
    ..TokenDef::DEFAULT
};
