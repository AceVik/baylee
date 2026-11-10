//! Land's Edge — {1}{R}{R} — World Enchantment
//! Oracle: Discard a card: If the discarded card was a land card, this enchantment deals 2 damage to target player or planeswalker. Any player may activate this ability.
//! Set: CHR #52 — Chronicles | Scryfall ID: 41798dd9-8ce8-4642-89c2-7356ea129d4e | Oracle ID: 82d58f2d-66a4-4154-9826-01ca8e8d32d0
use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAND_S_EDGE,
    oracle_id = "82d58f2d-66a4-4154-9826-01ca8e8d32d0",
    scryfall_id = "41798dd9-8ce8-4642-89c2-7356ea129d4e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Land's Edge",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD
    )],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(Discard(&Filter::Any)),
        &[Effect::IfCondition {
            condition: Condition::DiscardedForCost(&Filter::LAND),
            then: &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)
            }],
            otherwise: &[],
        }],
        target = Some(TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)),
        zone = ActivationZone::BattlefieldAnyPlayer
    )],
);
