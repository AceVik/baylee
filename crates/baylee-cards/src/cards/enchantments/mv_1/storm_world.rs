//! Storm World — {R} — World Enchantment
//! Oracle: At the beginning of each player's upkeep, this enchantment deals X damage to that player, where X is 4 minus the number of cards in their hand.
//! Set: ME3 #111 — Masters Edition III | Scryfall ID: ad43358a-6369-4b8b-a0b7-f8ba07c1bf39 | Oracle ID: 868f4ab2-a846-4ad0-8720-95fd234dd36b
// IMPLEMENTED — each upkeep, 4 minus the active player's hand
// (Amount::ConstantMinus) to that player.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STORM_WORLD,
    oracle_id = "868f4ab2-a846-4ad0-8720-95fd234dd36b",
    scryfall_id = "ad43358a-6369-4b8b-a0b7-f8ba07c1bf39",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Storm World",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::DealDamage {
            amount: Amount::ConstantMinus {
                constant: 4,
                subtract: &Amount::CountOf {
                    filter: &Filter::Any,
                    zone: ZoneSel::HandActivePlayer
                },
            },
            target: TargetSpec::Player(PlayerRel::ActivePlayer),
        }]
    )],
);
