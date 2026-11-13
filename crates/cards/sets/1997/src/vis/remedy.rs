//! Remedy — {1}{W} — Instant
//! Oracle: Prevent the next 5 damage that would be dealt this turn to any number of targets, divided as you choose.
//! Set: DMR #22 — Dominaria Remastered | Scryfall ID: 9700fe0e-bf71-463b-ad7b-34831c7f9e99 | Oracle ID: c644f190-63e5-4496-8612-bbe7be91afc9
// PARTIAL: prevents the next 5 damage that would be dealt to any single target this turn; dividing the 5 damage prevention among multiple targets is not yet supported.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: dividing damage prevention among any number of targets.
card!(
    index = index::REMEDY,
    oracle_id = "c644f190-63e5-4496-8612-bbe7be91afc9",
    scryfall_id = "9700fe0e-bf71-463b-ad7b-34831c7f9e99",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "prevents the next 5 damage that would be dealt to any single target this turn; dividing the 5 damage prevention among multiple targets is not yet supported"
    ),
    faces = &[face!(
        name = "Remedy",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PreventNextDamage {
            amount: Amount::Fixed(5),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
