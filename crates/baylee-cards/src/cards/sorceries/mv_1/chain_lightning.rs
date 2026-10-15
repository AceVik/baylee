//! Chain Lightning — {R} — Sorcery
//! Oracle: Chain Lightning deals 3 damage to any target. Then that player or that permanent's controller may pay {R}{R}. If the player does, they may copy this spell and may choose a new target for that copy.
//! Set: DMR #113 — Dominaria Remastered | Scryfall ID: 2105a0d9-ea2e-4ffc-be08-424b5617205d | Oracle ID: 8785f42f-87b9-4a0a-89ce-ea423649ba9c
// PARTIAL — all clauses are authored; battle damage and acceptance gates remain.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAIN_LIGHTNING,
    oracle_id = "8785f42f-87b9-4a0a-89ce-ea423649ba9c",
    scryfall_id = "2105a0d9-ea2e-4ffc-be08-424b5617205d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage =
        Coverage::Partial("battle damage and participant-controlled copy acceptance gates pending"),
    faces = &[face!(
        name = "Chain Lightning",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget,
            },
            Effect::PlayerMayPayManaThen {
                player: PlayerRel::TargetPlayerOrController,
                cost: mana!("{R}{R}"),
                effects: &[Effect::MayDoFor {
                    player: PlayerRel::TargetPlayerOrController,
                    effects: &[Effect::CopyResolvingSpell {
                        controller: PlayerRel::TargetPlayerOrController,
                    }],
                }],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )],
);
