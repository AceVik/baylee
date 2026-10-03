//! Guardian Angel — {X}{W} — Instant
//! Oracle: Prevent the next X damage that would be dealt to any target this turn. Until end of turn, you may pay {1} any time you could cast an instant. If you do, prevent the next 1 damage that would be dealt to that permanent or player this turn.
//! Set: SUM #21 — Summer Magic / Edgar | Scryfall ID: 071ca80b-62b4-40e6-81c6-fc5bd9019354 | Oracle ID: 1a91ca69-e890-41dc-866b-3aabf10c9a9c
// PARTIAL — special actions implemented; independent and client acceptance pending.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GUARDIAN_ANGEL,
    oracle_id = "1a91ca69-e890-41dc-866b-3aabf10c9a9c",
    scryfall_id = "071ca80b-62b4-40e6-81c6-fc5bd9019354",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage =
        Coverage::Partial("special actions implemented; independent and client acceptance pending"),
    faces = &[face!(
        name = "Guardian Angel",
        mana_cost = mana!("{X}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::PreventNextDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::X
            },
            Effect::GrantSpecialActionUntilEndOfTurn {
                timing: SpecialActionTiming::Priority,
                cost: SpecialActionCost::Mana(mana!("{1}")),
                effect: SpecialActionEffect::PreventNextDamage {
                    target: TargetSpec::AnyTarget,
                    amount: Amount::Fixed(1)
                },
            }
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    ),],
);
