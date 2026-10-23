//! Oath of Mages — {1}{R} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses target player who has more life than they do and is their opponent. The first player may have this enchantment deal 1 damage to the second player.
//! Set: EXO #90 — Exodus | Scryfall ID: ed8708d2-2c73-4da5-b6ff-41c083b59caa | Oracle ID: 2b749918-f425-4243-aabb-7350bb74403c
// PARTIAL: Triggers on your upkeep targeting any opponent to let you deal 1 damage to that player; triggering for each player and restricting target opponent to one who has more life are not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_MAGES,
    oracle_id = "2b749918-f425-4243-aabb-7350bb74403c",
    scryfall_id = "ed8708d2-2c73-4da5-b6ff-41c083b59caa",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Triggers on your upkeep targeting any opponent to let you deal 1 damage to that player; triggering for each player and restricting target opponent to one who has more life are not supported.",
    ),
    faces = &[face!(
        name = "Oath of Mages",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyOpponent,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    ),],
);
