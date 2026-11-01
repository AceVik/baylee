//! Constant Mists — {1}{G} — Instant
//! Oracle: Buyback—Sacrifice a land. (You may sacrifice a land in addition to any other costs as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Prevent all combat damage that would be dealt this turn.
//! Set: STH #104 — Stronghold | Scryfall ID: 97a8a5fe-0391-489b-9556-0a1bf7e1900d | Oracle ID: c850a29e-dc40-4ab6-89f1-d501a1a350d1
// PARTIAL: prevents all combat damage that would be dealt this turn; buyback—sacrifice a land is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONSTANT_MISTS,
    oracle_id = "c850a29e-dc40-4ab6-89f1-d501a1a350d1",
    scryfall_id = "97a8a5fe-0391-489b-9556-0a1bf7e1900d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "prevents all combat damage that would be dealt this turn; buyback—sacrifice a land is not supported"
    ),
    faces = &[face!(
        name = "Constant Mists",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::PreventAllCombatDamageThisTurn])],
);
