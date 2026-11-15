//! Life engines; costs and activations remain ordinary paid rules actions.
#[allow(clippy::wildcard_imports)]
use super::*;

const COIL_COMMON: AbilityDef = paid(cost!("{3}", PayLife(2)), &[Effect::draw(1)], ONCE);
const COIL_UNCOMMON: AbilityDef = paid(cost!("{2}", PayLife(2)), &[Effect::draw(1)], ONCE);
const COIL_RARE: AbilityDef = paid(cost!("{1}", PayLife(2)), &[Effect::draw(1)], ONCE);
const COIL_MYTHIC: AbilityDef = paid(cost!("{1}", PayLife(2)), &[Effect::draw(1)], UNLIMITED);
const DRAW_DRAIN: AbilityDef = triggered!(
    Trigger::Draws(PlayerRel::You),
    &[Effect::LoseLife {
        target: PlayerRel::EachOpponent,
        amount: Amount::Fixed(1)
    }]
);

/// Common heals for one on your upkeep.
pub static TOWER_RUN_CHALICE_COMMON: TokenDef = TokenDef {
    name: "Spring Chalice — Common",
    abilities: &[UPKEEP_LIFE],
    ..TokenDef::DEFAULT
};
/// Uncommon heals for two and rewards draws with life.
pub static TOWER_RUN_CHALICE_UNCOMMON: TokenDef = TokenDef {
    name: "Spring Chalice — Uncommon",
    abilities: &[upkeep(&[Effect::gain_life(2)]), DRAW_LIFE],
    ..TokenDef::DEFAULT
};
/// Rare heals for three and offers a paid life-to-card conversion.
pub static TOWER_RUN_CHALICE_RARE: TokenDef = TokenDef {
    name: "Spring Chalice — Rare",
    abilities: &[upkeep(&[Effect::gain_life(3)]), DRAW_LIFE, COIL_UNCOMMON],
    ..TokenDef::DEFAULT
};
/// Mythic heals for five and draws an additional card each upkeep.
pub static TOWER_RUN_CHALICE_MYTHIC: TokenDef = TokenDef {
    name: "Spring Chalice — Mythic Rare",
    abilities: &[
        upkeep(&[Effect::gain_life(5)]),
        DRAW_LIFE,
        COIL_UNCOMMON,
        upkeep(&[Effect::draw(1)]),
    ],
    ..TokenDef::DEFAULT
};

/// Common heals whenever your creatures enter.
pub static TOWER_RUN_HEARTH_COMMON: TokenDef = TokenDef {
    name: "Hearthstone — Common",
    abilities: &[ENTER_LIFE],
    ..TokenDef::DEFAULT
};
/// Uncommon doubles the heal and grows a Plant each upkeep.
pub static TOWER_RUN_HEARTH_UNCOMMON: TokenDef = TokenDef {
    name: "Hearthstone — Uncommon",
    abilities: &[
        super::super::TOWER_HEARTH_UNCOMMON.abilities[0],
        UPKEEP_PLANT,
    ],
    ..TokenDef::DEFAULT
};
/// Rare heals for three and toughens your tokens.
pub static TOWER_RUN_HEARTH_RARE: TokenDef = TokenDef {
    name: "Hearthstone — Rare",
    abilities: &[
        super::super::TOWER_HEARTH_RARE.abilities[0],
        UPKEEP_PLANT,
        TOKEN_GUARD,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic heals for four and drains opponents on each life-gain event.
pub static TOWER_RUN_HEARTH_MYTHIC: TokenDef = TokenDef {
    name: "Hearthstone — Mythic Rare",
    abilities: &[
        super::super::TOWER_HEARTH_MYTHIC.abilities[0],
        UPKEEP_PLANT,
        TOKEN_GUARD,
        LIFE_DRAIN,
    ],
    ..TokenDef::DEFAULT
};

const LIFE_CHARGE: AbilityDef = triggered!(Trigger::GainsLife(PlayerRel::You), &[CHARGE_ONE]);
const SHOT_COST: Cost = cost!(RemoveCounterSelf {
    kind: CounterKind::Charge,
    n: 2
});
const fn shot(effects: &'static [Effect], limit: ActivationLimit) -> AbilityDef {
    activated!(
        SHOT_COST,
        effects,
        zone = ActivationZone::Command,
        limit = limit,
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )
}
/// Common stores life-gain events and fires once each turn.
pub static TOWER_RUN_AMBER_COMMON: TokenDef = TokenDef {
    name: "Blood Amber — Common",
    abilities: &[
        LIFE_CHARGE,
        shot(
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            }],
            ONCE,
        ),
    ],
    ..TokenDef::DEFAULT
};
/// Uncommon fires for two and charges each upkeep.
pub static TOWER_RUN_AMBER_UNCOMMON: TokenDef = TokenDef {
    name: "Blood Amber — Uncommon",
    abilities: &[
        LIFE_CHARGE,
        shot(
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget,
            }],
            ONCE,
        ),
        UPKEEP_CHARGE,
    ],
    ..TokenDef::DEFAULT
};
/// Rare fires for three and heals with each draw.
pub static TOWER_RUN_AMBER_RARE: TokenDef = TokenDef {
    name: "Blood Amber — Rare",
    abilities: &[
        LIFE_CHARGE,
        shot(
            &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget,
            }],
            ONCE,
        ),
        UPKEEP_CHARGE,
        DRAW_LIFE,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic fires for four without a turn cap and adds life-gain drain.
pub static TOWER_RUN_AMBER_MYTHIC: TokenDef = TokenDef {
    name: "Blood Amber — Mythic Rare",
    abilities: &[
        LIFE_CHARGE,
        shot(
            &[Effect::DealDamage {
                amount: Amount::Fixed(4),
                target: TargetSpec::AnyTarget,
            }],
            UNLIMITED,
        ),
        UPKEEP_CHARGE,
        DRAW_LIFE,
        LIFE_DRAIN,
    ],
    ..TokenDef::DEFAULT
};

/// Common trades three mana and two life for a card, once each turn.
pub static TOWER_RUN_COIL_COMMON: TokenDef = TokenDef {
    name: "Life Coil — Common",
    abilities: &[COIL_COMMON],
    ..TokenDef::DEFAULT
};
/// Uncommon reduces mana to two and heals with each draw.
pub static TOWER_RUN_COIL_UNCOMMON: TokenDef = TokenDef {
    name: "Life Coil — Uncommon",
    abilities: &[COIL_UNCOMMON, DRAW_LIFE],
    ..TokenDef::DEFAULT
};
/// Rare reduces mana to one and adds upkeep healing.
pub static TOWER_RUN_COIL_RARE: TokenDef = TokenDef {
    name: "Life Coil — Rare",
    abilities: &[COIL_RARE, DRAW_LIFE, upkeep(&[Effect::gain_life(2)])],
    ..TokenDef::DEFAULT
};
/// Mythic removes the turn cap and turns draws into opposing life loss.
pub static TOWER_RUN_COIL_MYTHIC: TokenDef = TokenDef {
    name: "Life Coil — Mythic Rare",
    abilities: &[
        COIL_MYTHIC,
        DRAW_LIFE,
        upkeep(&[Effect::gain_life(2)]),
        DRAW_DRAIN,
    ],
    ..TokenDef::DEFAULT
};
