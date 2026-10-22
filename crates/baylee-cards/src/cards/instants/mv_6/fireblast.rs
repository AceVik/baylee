//! Fireblast — {4}{R}{R} — Instant
//! Oracle: You may sacrifice two Mountains rather than pay this spell's mana cost.
//! Oracle: Fireblast deals 4 damage to any target.
//! Set: DMR #119 — Dominaria Remastered | Scryfall ID: a0ab67cc-b553-4451-9cda-0e5fe3303940 | Oracle ID: 9dd7f27a-e862-47d7-9158-034cf4d353b8
// PARTIAL — Sacrificing two Mountains rather than paying this spell's mana cost is not yet supported by cast_wizard::paid_as_an_alternative_cost; in play, Fireblast is cast for {4}{R}{R} and deals 4 damage to any target.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIREBLAST,
    oracle_id = "9dd7f27a-e862-47d7-9158-034cf4d353b8",
    scryfall_id = "a0ab67cc-b553-4451-9cda-0e5fe3303940",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Sacrificing two Mountains rather than paying this spell's mana cost is not yet supported by cast_wizard::paid_as_an_alternative_cost; in play, Fireblast is cast for {4}{R}{R} and deals 4 damage to any target.",
    ),
    faces = &[face!(
        name = "Fireblast",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(4),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
