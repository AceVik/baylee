//! Jovial Evil — {2}{B} — Sorcery
//! Oracle: Jovial Evil deals X damage to target opponent, where X is twice the number of white creatures that player controls.
//! Set: LEG #109 — Legends | Scryfall ID: c993c74c-a574-423b-81c8-96b0a7a6e529 | Oracle ID: 274c5367-f02e-44a0-be8a-5ed03c831bda
// PARTIAL — the whole spell is off the card: X cannot be computed from the
// targeted player's board.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JOVIAL_EVIL,
    oracle_id = "274c5367-f02e-44a0-be8a-5ed03c831bda",
    scryfall_id = "c993c74c-a574-423b-81c8-96b0a7a6e529",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Jovial Evil",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "X cannot be computed: no Amount doubles a counted quantity and no \
         Filter reads the permanents controlled by the player the first target \
         names",
    ),
    // NOT SUPPORTED: "Jovial Evil deals X damage to target opponent, where X
    // is twice the number of white creatures that player controls." — the
    // target is writable (`TargetSpec::AnyOpponent`), but no `Amount` doubles
    // a counted quantity (`Amount::DoubleX` doubles the announced X, and this
    // spell announces none), and no `Filter` reads the board of the player a
    // target names, so X has nothing to count.
    abilities = &[],
);
