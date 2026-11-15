//! Original Root Reservoir: entering creature tokens store charges for green mana.
use baylee_cards_dsl::prelude::*;

const CHARGE: AbilityDef = triggered!(
    Trigger::EntersBattlefield(&f!(your token CREATURE)),
    &[Effect::AddCounter {
        kind: CounterKind::Charge,
        amount: Amount::Fixed(1)
    }]
);

/// Common: tap and remove three charges to add one green mana.
pub static TOWER_ROOTS_COMMON: TokenDef = TokenDef {
    name: "Root Reservoir — Common",
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
            &[Effect::mana(ManaColor::Green, 1)]
        ),
    ],
    ..TokenDef::DEFAULT
};

/// Uncommon: tap and remove two charges to add one green mana.
pub static TOWER_ROOTS_UNCOMMON: TokenDef = TokenDef {
    name: "Root Reservoir — Uncommon",
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
            &[Effect::mana(ManaColor::Green, 1)]
        ),
    ],
    ..TokenDef::DEFAULT
};

/// Rare: remove two charges to add one green mana, without tapping.
pub static TOWER_ROOTS_RARE: TokenDef = TokenDef {
    name: "Root Reservoir — Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[
        CHARGE,
        mana_ability!(
            cost!(RemoveCounterSelf {
                kind: CounterKind::Charge,
                n: 2
            }),
            &[Effect::mana(ManaColor::Green, 1)]
        ),
    ],
    ..TokenDef::DEFAULT
};

/// Mythic Rare: remove one charge to add one green mana, without tapping.
pub static TOWER_ROOTS_MYTHIC: TokenDef = TokenDef {
    name: "Root Reservoir — Mythic Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[
        CHARGE,
        mana_ability!(
            cost!(RemoveCounterSelf {
                kind: CounterKind::Charge,
                n: 1
            }),
            &[Effect::mana(ManaColor::Green, 1)]
        ),
    ],
    ..TokenDef::DEFAULT
};
