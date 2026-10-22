//! Psychic Transfer — {4}{U} — Sorcery
//! Oracle: If the difference between your life total and target player's life total is 5 or less, exchange life totals with that player.
//! Set: S99 #46 — Starter 1999 | Scryfall ID: ac6d7d5a-697b-4251-bc18-4dcfe0aa5cc6 | Oracle ID: 5554d422-6513-4c7e-866d-9d5f0403fc80
// PARTIAL: Condition has no absolute life-total difference comparison ("difference between your life total and target player's life total is 5 or less"); casting this spell exchanges your life total with target player's life total

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PSYCHIC_TRANSFER,
    oracle_id = "5554d422-6513-4c7e-866d-9d5f0403fc80",
    scryfall_id = "ac6d7d5a-697b-4251-bc18-4dcfe0aa5cc6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Condition has no absolute life-total difference comparison (\"difference between your life total and target player's life total is 5 or less\"); casting this spell exchanges your life total with target player's life total"
    ),
    faces = &[face!(
        name = "Psychic Transfer",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::ExchangeLifeTotals {
            with: PlayerRel::Chosen,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
