//! Cabal Ritual — {1}{B} — Instant
//! Oracle: Add {B}{B}{B}.
//! Oracle: Threshold — Add {B}{B}{B}{B}{B} instead if there are seven or more cards in your graveyard.
//! Set: VMA #106 — Vintage Masters | Scryfall ID: a5d85875-22da-4054-ae42-e85b472a6d5d | Oracle ID: 5b5bf1fa-6502-4790-b66b-f0f8504ebc7c
// IMPLEMENTED — three black, or five with threshold.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CABAL_RITUAL,
    oracle_id = "5b5bf1fa-6502-4790-b66b-f0f8504ebc7c",
    scryfall_id = "a5d85875-22da-4054-ae42-e85b472a6d5d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cabal Ritual",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
    // "Instead", so the two amounts are the branches of one effect and not
    // three mana plus a conditional two: a replacement of the whole line is
    // what the word means, and writing it as an add-on would make a card
    // that adds five where a `Effect::mana` reduction would later read three.
    //
    // Threshold is an ability word with no rules meaning, so the count is
    // written out and nothing here reads the word — the same bargain
    // Barbarian Ring makes with the same number.
    abilities = &[spell!(&[Effect::IfCondition {
        condition: Condition::GraveyardCountAtLeast(7),
        then: &[Effect::mana(ManaColor::Black, 5)],
        otherwise: &[Effect::mana(ManaColor::Black, 3)],
    }])],
);
