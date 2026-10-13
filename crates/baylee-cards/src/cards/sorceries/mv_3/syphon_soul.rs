//! Syphon Soul — {2}{B} — Sorcery
//! Oracle: Syphon Soul deals 2 damage to each other player. You gain life equal to the damage dealt this way.
//! Set: CNS #127 — Conspiracy | Scryfall ID: dc308882-1c12-4da0-8eae-caf4cc3d43e1 | Oracle ID: d47e55a6-7a16-4ed9-a94b-f1575588ee9e
// IMPLEMENTED — 2 damage to each other player, then you gain life equal to the
// damage dealt this way.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SYPHON_SOUL,
    oracle_id = "d47e55a6-7a16-4ed9-a94b-f1575588ee9e",
    scryfall_id = "dc308882-1c12-4da0-8eae-caf4cc3d43e1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Syphon Soul",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[
        // "each other player", not "each opponent": a teammate is another
        // player and no opponent (CR 102.3).
        Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::EachOtherPlayer),
        },
        Effect::GainLife {
            amount: Amount::ThisWay(ThisWay::DamageDealt),
        },
    ])],
);
