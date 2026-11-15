//! Original Harvest Wheel: dying creature tokens store charges for colorless mana.
use baylee_cards_dsl::prelude::*;

const CHARGE: AbilityDef = triggered!(
    Trigger::Dies(&f!(your token CREATURE)),
    &[Effect::AddCounter {
        kind: CounterKind::Charge,
        amount: Amount::Fixed(1)
    }]
);

/// Common: tap and remove three charges to add two colorless mana.
pub static TOWER_HARVEST_COMMON: TokenDef = TokenDef {
    name: "Harvest Wheel — Common",
    types: TypeSet::ARTIFACT,
    abilities: &[
        CHARGE,
        mana_ability!(
            cost!(
                TapSelf,
                RemoveCounterSelf {
                    kind: CounterKind::Charge,
                    n: 3
                }
            ),
            &[Effect::mana(ManaColor::Colorless, 2)]
        ),
    ],
    ..TokenDef::DEFAULT
};

/// Uncommon: tap and remove two charges to add two colorless mana.
pub static TOWER_HARVEST_UNCOMMON: TokenDef = TokenDef {
    name: "Harvest Wheel — Uncommon",
    types: TypeSet::ARTIFACT,
    abilities: &[
        CHARGE,
        mana_ability!(
            cost!(
                TapSelf,
                RemoveCounterSelf {
                    kind: CounterKind::Charge,
                    n: 2
                }
            ),
            &[Effect::mana(ManaColor::Colorless, 2)]
        ),
    ],
    ..TokenDef::DEFAULT
};

/// Rare: tap and remove one charge to add two colorless mana.
pub static TOWER_HARVEST_RARE: TokenDef = TokenDef {
    name: "Harvest Wheel — Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[
        CHARGE,
        mana_ability!(
            cost!(
                TapSelf,
                RemoveCounterSelf {
                    kind: CounterKind::Charge,
                    n: 1
                }
            ),
            &[Effect::mana(ManaColor::Colorless, 2)]
        ),
    ],
    ..TokenDef::DEFAULT
};

/// Mythic Rare: remove one charge to add three colorless mana, without tapping.
pub static TOWER_HARVEST_MYTHIC: TokenDef = TokenDef {
    name: "Harvest Wheel — Mythic Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[
        CHARGE,
        mana_ability!(
            cost!(RemoveCounterSelf {
                kind: CounterKind::Charge,
                n: 1
            }),
            &[Effect::mana(ManaColor::Colorless, 3)]
        ),
    ],
    ..TokenDef::DEFAULT
};
