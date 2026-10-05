//! Chain Lightning — {R} — Sorcery
//! Oracle: Chain Lightning deals 3 damage to any target. Then that player or that permanent's controller may pay {R}{R}. If the player does, they may copy this spell and may choose a new target for that copy.
//! Set: DMR #113 — Dominaria Remastered | Scryfall ID: 2105a0d9-ea2e-4ffc-be08-424b5617205d | Oracle ID: 8785f42f-87b9-4a0a-89ce-ea423649ba9c
// PARTIAL — the 3 damage is written; the pay-{R}{R}-to-copy clause is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAIN_LIGHTNING,
    oracle_id = "8785f42f-87b9-4a0a-89ce-ea423649ba9c",
    scryfall_id = "2105a0d9-ea2e-4ffc-be08-424b5617205d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no card-level effect copies the resolving spell under the paying \
         player's control, and no copy effect hands the new-target choice to \
         that player"
    ),
    faces = &[face!(
        name = "Chain Lightning",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
    // NOT SUPPORTED: "Then that player or that permanent's controller may pay
    // {R}{R}. If the player does, they may copy this spell and may choose a
    // new target for that copy." — `Effect::CopyThisSpell` is the engine's
    // replicate trigger and is written by no card; `Effect::CopyTargetSpell`
    // copies a targeted spell, and this spell on the stack is not a legal
    // target of itself. `PlayerMayPayManaThen` could ask a seat, but the copy
    // its `effects` would make is controlled by the resolving ability's
    // controller, not by the player who paid, and that player is the one the
    // card lets choose the copy's new target.
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::AnyTarget
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )],
);
