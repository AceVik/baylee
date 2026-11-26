//! Chain of Plasma — {1}{R} — Instant
//! Oracle: Chain of Plasma deals 3 damage to any target. Then that player or that permanent's controller may discard a card. If the player does, they may copy this spell and may choose a new target for that copy.
//! Set: ONS #193 — Onslaught | Scryfall ID: f94aa774-9036-4016-8880-4bde2710cb90 | Oracle ID: 029d08cb-da4d-4f4e-b667-62712af63260
// IMPLEMENTED — 3 damage to any target with optional discard-to-copy chain.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAIN_OF_PLASMA,
    oracle_id = "029d08cb-da4d-4f4e-b667-62712af63260",
    scryfall_id = "f94aa774-9036-4016-8880-4bde2710cb90",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chain of Plasma",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget,
            },
            Effect::PlayerMayPayCostThen {
                player: PlayerRel::TargetPlayerOrController,
                cost: &CostPart::Discard(&Filter::Any),
                effect: &Effect::MayDoFor {
                    player: PlayerRel::TargetPlayerOrController,
                    effects: &[Effect::CopySpellOrAbility {
                        target: TargetSpec::ThisObject,
                        controller: PlayerRel::TargetPlayerOrController,
                        amount: Amount::Fixed(1),
                        mods: &[],
                        may_choose_new_targets: true,
                    }],
                },
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
