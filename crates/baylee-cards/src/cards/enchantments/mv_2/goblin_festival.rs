//! Goblin Festival — {1}{R} — Enchantment
//! Oracle: {2}: This enchantment deals 1 damage to any target. Flip a coin. If you lose the flip, choose one of your opponents. That player gains control of this enchantment.
//! Set: UDS #83 — Urza's Destiny | Scryfall ID: ac067eb8-427f-4bfa-b392-0bb41ac8370e | Oracle ID: de21b90f-a268-46d6-b977-d5b0856176f3
// IMPLEMENTED — {2} activated ability deals 1 damage to any target, flips a coin, and on a lost flip has the controller choose an opponent who gains control of this enchantment.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GOBLIN_FESTIVAL,
    oracle_id = "de21b90f-a268-46d6-b977-d5b0856176f3",
    scryfall_id = "ac067eb8-427f-4bfa-b392-0bb41ac8370e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Festival",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{2}"),
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            },
            Effect::FlipCoin {
                won: &[],
                lost: &[
                    Effect::ChoosePlayerFrom {
                        among: PlayerRel::EachOpponent,
                    },
                    Effect::GainControlAll {
                        filter: &Filter::This,
                        to: PlayerRel::Chosen,
                    },
                ],
            },
        ],
        target = Some(TargetSpec::AnyTarget),
    )],
);
