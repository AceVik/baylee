//! Lich — {B}{B}{B}{B} — Enchantment
//! Oracle: As this enchantment enters, you lose life equal to your life total.
//! Oracle: You don't lose the game for having 0 or less life.
//! Oracle: If you would gain life, draw that many cards instead.
//! Oracle: Whenever you're dealt damage, sacrifice that many nontoken permanents. If you can't, you lose the game.
//! Oracle: When this enchantment is put into a graveyard from the battlefield, you lose the game.
//! Set: ME4 #89 — Masters Edition IV | Scryfall ID: d4d7d1fd-4a5e-4cc3-8056-12cae084cc6a | Oracle ID: 5b7515f2-7a5a-4e2a-9784-6cbacd768172
// PARTIAL — not losing the game at 0 life, life gain as draws and damage as
// sacrifices are not in the engine.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LICH,
    oracle_id = "5b7515f2-7a5a-4e2a-9784-6cbacd768172",
    scryfall_id = "d4d7d1fd-4a5e-4cc3-8056-12cae084cc6a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "not losing the game at 0 life, life gain as draws and damage as sacrifices are not in the engine"
    ),
    faces = &[face!(
        name = "Lich",
        mana_cost = mana!("{B}{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: As this enchantment enters, you lose life equal to your life total.
    // NOT SUPPORTED: You don't lose the game for having 0 or less life.
    // NOT SUPPORTED: If you would gain life, draw that many cards instead.
    // NOT SUPPORTED: Whenever you're dealt damage, sacrifice that many nontoken
    // permanents. If you can't, you lose the game.
    // NOT SUPPORTED: When this enchantment is put into a graveyard from the battlefield,
    // you lose the game.
);
