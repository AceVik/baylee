//! Abeyance — {1}{W} — Instant
//! Oracle: Until end of turn, target player can't cast instant or sorcery spells, and that player can't activate abilities that aren't mana abilities.
//! Oracle: Draw a card.
//! Set: WTH #1 — Weatherlight | Scryfall ID: 125a355d-bfcf-4125-aa6c-35e7dea6f63e | Oracle ID: 6300de53-6e71-4f0e-87e1-b0acd25c59a8
// NOT SUPPORTED: preventing target player from casting instant or sorcery spells or activating non-mana abilities until end of turn is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ABEYANCE,
    oracle_id = "6300de53-6e71-4f0e-87e1-b0acd25c59a8",
    scryfall_id = "125a355d-bfcf-4125-aa6c-35e7dea6f63e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "targets a player and draws a card; preventing that player from casting instant or sorcery spells or activating non-mana abilities until end of turn is not yet modeled",
    ),
    faces = &[face!(
        name = "Abeyance",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::draw(1)],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
