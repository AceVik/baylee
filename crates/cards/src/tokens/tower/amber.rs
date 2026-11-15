//! Original Blood Amber: life-gain events, not life points, store one charge.
use baylee_cards_dsl::prelude::*;

const CHARGE: AbilityDef = triggered!(
    Trigger::GainsLife(PlayerRel::You),
    &[Effect::AddCounter {
        kind: CounterKind::Charge,
        amount: Amount::Fixed(1)
    }]
);

/// Common: tap and remove two charges for one damage to any target.
pub static TOWER_AMBER_COMMON: TokenDef = TokenDef {
    name: "Blood Amber — Common",
    types: TypeSet::ARTIFACT,
    abilities: &[
        CHARGE,
        activated!(
            cost!(
                TapSelf,
                RemoveCounterSelf {
                    kind: CounterKind::Charge,
                    n: 2
                }
            ),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
    ],
    ..TokenDef::DEFAULT
};

/// Uncommon: tap and remove two charges for two damage to any target.
pub static TOWER_AMBER_UNCOMMON: TokenDef = TokenDef {
    name: "Blood Amber — Uncommon",
    types: TypeSet::ARTIFACT,
    abilities: &[
        CHARGE,
        activated!(
            cost!(
                TapSelf,
                RemoveCounterSelf {
                    kind: CounterKind::Charge,
                    n: 2
                }
            ),
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
    ],
    ..TokenDef::DEFAULT
};

/// Rare: tap and remove two charges for three damage to any target.
pub static TOWER_AMBER_RARE: TokenDef = TokenDef {
    name: "Blood Amber — Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[
        CHARGE,
        activated!(
            cost!(
                TapSelf,
                RemoveCounterSelf {
                    kind: CounterKind::Charge,
                    n: 2
                }
            ),
            &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
    ],
    ..TokenDef::DEFAULT
};

/// Mythic Rare: remove two charges for three damage, without tapping.
pub static TOWER_AMBER_MYTHIC: TokenDef = TokenDef {
    name: "Blood Amber — Mythic Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[
        CHARGE,
        activated!(
            cost!(RemoveCounterSelf {
                kind: CounterKind::Charge,
                n: 2
            }),
            &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
    ],
    ..TokenDef::DEFAULT
};
