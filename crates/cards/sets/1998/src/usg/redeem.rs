//! Redeem — {1}{W} — Instant
//! Oracle: Prevent all damage that would be dealt this turn to up to two target creatures.
//! Set: USG #33 — Urza's Saga | Scryfall ID: 05a7756d-df25-4969-96ad-b006df09788b | Oracle ID: c2032f88-a000-4ed2-a7e5-61d29723d45e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REDEEM,
    oracle_id = "c2032f88-a000-4ed2-a7e5-61d29723d45e",
    scryfall_id = "05a7756d-df25-4969-96ad-b006df09788b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Redeem",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::continuous(
            &Filter::This,
            Modifier::PreventDamageFrom(&Filter::Any),
            Duration::UntilEndOfTurn,
        )],
        targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 2)),
    ),],
);
